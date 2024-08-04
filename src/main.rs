use std::rc::Rc;

use color_eyre::{eyre::WrapErr, owo_colors::OwoColorize, Result};
use rand::{thread_rng, Rng};
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::{block::Title, Block, Borders, Paragraph, Widget},
    Frame,
};

mod errors;
mod tui;

fn main() -> Result<()> {
    let mut terminal = tui::init()?;
    App::default().run(&mut terminal)?;
    tui::restore()?;
    Ok(())
}

#[derive(Debug)]
pub enum Mode {
    Random,
    Chars(u8),
    Words(u8),
}

impl Default for Mode {
    fn default() -> Self {
        Self::Random
    }
}

#[derive(Debug, Default)]
pub enum SpanType {
    #[default]
    DEFAULT,
    HIT,
    MISS,
}

#[derive(Debug, Default)]
pub struct TextSpan<'a> {
    span_type: SpanType,
    span: Span<'a>,
}

impl<'a> TextSpan<'a> {
    pub fn new(span_type: SpanType, span: Span<'a>) -> Self {
        Self { span_type, span }
    }

    pub fn default_with_text(text: String) -> Self {
        let mut def = Self::default();
        def.span.content = text.into();
        def
    }

    pub fn hit(value: String) -> Self {
        Self {
            span_type: SpanType::HIT,
            span: value.green(),
        }
    }

    pub fn miss(value: String) -> Self {
        Self {
            span_type: SpanType::MISS,
            span: value.red(),
        }
    }
}

#[derive(Debug, Default)]
pub struct App<'a> {
    mode: Mode,
    wins: u8,
    fails: u8,
    remainder: TextSpan<'a>,
    spans: Vec<TextSpan<'a>>,
    exit: bool,
    miss_this_round: bool,
}

const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
const ALPHABET: [&str; 26] = [
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s",
    "t", "u", "v", "w", "x", "y", "z",
];
const SPECIALS: [&str; 31] = [
    "!", "@", "#", "$", "%", "^", "&", "*", "(", ")", "-", "_", "+", "=", "{", "}", "[", "]", "|",
    "\\", ":", ";", "\"", "\"", "<", ">", ",", ".", "/", "?", "`",
];

impl App<'_> {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        let res = self.next_round();
        if res.is_err() {
            println!("{:?}", res);
            self.exit();
        }

        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events().wrap_err("handle events failed")?;
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => self
                .handle_key_event(key_event)
                .wrap_err_with(|| format!("handling key event failed:\n{key_event:#?}")),
            _ => Ok(()),
        }
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Esc => self.exit(),
            KeyCode::Char(v) => {
                let is_hit = self.remainder.span.content.starts_with(v);

                if is_hit {
                    let new_remainder = self.remainder.span.content.replacen(v, "", 1);

                    if new_remainder.is_empty() {
                        let res = self.count(self.miss_this_round);
                        if res.is_err() {
                            self.exit_error("Counting up failed. Exiting");
                        }

                        let res = self.next_round();
                        if res.is_err() {
                            self.exit_error("Generating the next round failed");
                        }

                        return Ok(());
                    }

                    if self.spans.is_empty() {
                        self.spans.push(TextSpan::hit(v.to_string()));
                    } else {
                        let last = self.spans.pop();
                        if last.is_some() {
                            self.spans.push(TextSpan::hit(format!(
                                "{}{}",
                                last.unwrap().span.content,
                                v
                            )));
                        } else {
                            self.exit_error("last is None; Exiting");
                        }
                    }

                    // I don't get why this is considered a "move out of the span"
                    // I'm trying to replace the contents of the span with a cloned
                    // String?
                    // self.remainder
                    //     .span
                    //     .content(self.remainder.span.content.replacen(v, "", 1));

                    //     Creating a new object and not just modifying the
                    //     existing one works, but is is best practice?
                    self.remainder.span = Span::default().content(new_remainder);
                } else {
                    self.miss_this_round = true;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn exit_error(&mut self, msg: &str) {
        println!("Exiting with error: {}", msg);
        self.exit = true;
    }
    fn count(&mut self, fail: bool) -> Result<()> {
        if fail {
            self.fails += 1;
        } else {
            self.wins += 1;
        }
        Ok(())
    }

    fn next_round(&mut self) -> Result<()> {
        let mut rng = thread_rng();
        let mut a: String = ALPHABET[rng.gen_range(0..ALPHABET.len())].to_string();
        let b: String = ALPHABET[rng.gen_range(0..ALPHABET.len())].to_string();
        a.push_str(&b);
        self.spans.clear();
        self.remainder = TextSpan::default_with_text(a);
        self.miss_this_round = false;
        Ok(())
    }

    fn build_main_layout(area: Rect) -> Rc<[Rect]> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(7), Constraint::Length(2)])
            .margin(1)
            .split(area)
    }

    fn build_stats_layout(area: Rect) -> Rc<[Rect]> {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(45),
                Constraint::Percentage(10),
                Constraint::Percentage(45),
            ])
            .margin(2)
            .split(area)
    }

    fn render_stats_block(layout: Rect, buf: &mut Buffer, title: &str, value: &u8) {
        let title = Title::from(title.bold());
        let text = Text::from(vec![Line::from(value.to_string().yellow().bold())]);
        let block = Block::default()
            .title(title.alignment(Alignment::Center))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .borders(Borders::ALL);
        Paragraph::new(text)
            .centered()
            .block(block)
            .render(layout, buf);
    }

    fn render_input_box(&self, area: Rect, buf: &mut Buffer) {
        let mut sspans: Vec<Span> = vec![];
        self.spans.iter().for_each(|line| {
            sspans.push(line.span.clone());
        });
        sspans.push(self.remainder.span.clone());

        let text = Line::from(sspans);

        let h_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Min(1),
                Constraint::Length(text.width() as u16),
                Constraint::Min(1),
            ])
            .split(area);

        let block = Block::default().bold();
        Paragraph::new(text).block(block).render(h_layout[1], buf);
    }
}

impl Widget for &App<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let main = App::build_main_layout(area);
        let layout_stats = App::build_stats_layout(main[0]);

        App::render_stats_block(layout_stats[0], buf, " WINS ", &self.wins);
        App::render_stats_block(layout_stats[2], buf, " FAILS ", &self.fails);

        self.render_input_box(main[1], buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_key_event() {
        let mut app = App::default();
        let res = app.next_round();
        assert!(res.is_ok());
        assert!(app.remainder.span.content.len() == 2);

        // Why does setting the content in the span move the value?
        // Diagnostics:
        // 1. `app.remainder.span` partially moved due to this method call [E0382]
        // 2. you can `clone` the value and consume it, but this might not be your desired behavior: `.clone()` [E0382]
        // let _ = app.remainder.span.content("ab");

        // Replacing the whole object works
        // Seems wasteful if I just want to replace existing content.
        app.remainder.span = Span::default().content("ab");

        // 1. borrow of partially moved value: `app`
        //    partial move occurs because `app.remainder.span` has type `ratatui::prelude::Span<'_>`, which does not implement the `Copy` trait [E0382]
        let _ = app.handle_key_event(KeyCode::Char('a').into());
        assert!(!app.miss_this_round);
        assert!(app.remainder.span.content == "b");

        let _ = app.handle_key_event(KeyCode::Char('c').into());
        assert!(app.miss_this_round);
        assert!(app.remainder.span.content == "b");

        let _ = app.handle_key_event(KeyCode::Char('b').into());
        assert!(app.wins == 0);
        assert!(app.fails == 1);
        assert!(app.remainder.span.content.len() == 2);

        // Can't get the value of content? Not even when I clone it?
        // let c = app.remainder.span.content.to_string().clone();
        // assert_eq!(c , "b");

        let mut app = App::default();
        app.handle_key_event(KeyCode::Esc.into()).unwrap();
        assert!(app.exit);
    }
}
