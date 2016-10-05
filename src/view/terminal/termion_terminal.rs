use super::Terminal;
use std::cell::RefCell;
use std::fmt::Display;
use std::io::Stdout;
use scribe::buffer::Position;
use termion;
use termion::color::{Bg, Fg};
use termion::cursor;
use termion::input::{Keys, TermRead};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::style;
use std::io::{Stdin, stdin, stdout, Write};
use view::{Colors, Style};
use input::Key;

pub struct TermionTerminal {
    input: Keys<Stdin>,
    output: Option<RefCell<RawTerminal<Stdout>>>,
}

impl TermionTerminal {
    pub fn new() -> TermionTerminal {
        TermionTerminal {
            input: stdin().keys(),
            output: Some(
                RefCell::new(
                    create_output_instance()
                )
            )
        }
    }

    fn reset_style(&self) {
        if let Some(ref output) = self.output {
            write!(output.borrow_mut(), "{}", style::Reset);
        }
    }
}


impl Terminal for TermionTerminal {
    fn listen(&mut self) -> Option<Key> {
        self.input.next().and_then(|k| k.ok())
    }

    fn clear(&self) {
        self.output.as_ref().map(|t| write!(t.borrow_mut(), "{}", termion::clear::All));
    }

    fn present(&self) {
        self.output.as_ref().map(|t| t.borrow_mut().flush());
    }

    fn width(&self) -> usize {
        let (width, _) = terminal_size();

        width
    }

    fn height(&self) -> usize {
        let (_, height) = terminal_size();

        height
    }

    fn set_cursor(&self, position: Option<Position>) {
        self.output.as_ref().map(|t| {
            match position {
                Some(ref pos) => write!(t.borrow_mut(), "{}", cursor_position(pos)),
                None => write!(t.borrow_mut(), "{}", cursor::Hide),
            }
        });
    }

    fn print(&self, x: usize, y: usize, style: Style, colors: Colors, content: &Display) {
        if let Colors::Custom(fg, bg) = colors {
            self.reset_style();

            if let Some(ref output) = self.output {
                write!(
                    output.borrow_mut(),
                    "{}{}{}{}{}{}",
                    cursor_position(&Position{ line: y, offset: x }),
                    style::Reset,
                    map_style(style).unwrap_or(Box::new(style::Reset)),
                    Fg(fg),
                    Bg(bg),
                    content
                );
            }
        }
    }

    fn stop(&mut self) {
        // Terminal destructor cleans up for us.
        self.output = None;
    }

    fn start(&mut self) {
        // We don't want to initialize the terminal twice.
        if self.output.is_none() {
            self.output = Some(RefCell::new(create_output_instance()));
        }
    }
}

fn cursor_position(position: &Position) -> cursor::Goto {
    cursor::Goto(
        (position.offset + 1) as u16,
        (position.line + 1) as u16
    )
}

fn terminal_size() -> (usize, usize) {
    termion::terminal_size()
        .map(|(x,y)| (x as usize, y as usize))
        .unwrap_or((0, 0))
}

fn create_output_instance() -> RawTerminal<Stdout> {
    stdout().into_raw_mode().unwrap()
}

fn map_style(style: Style) -> Option<Box<Display>> {
    match style {
        Style::Default => None,
        Style::Bold => Some(Box::new(style::Bold)),
        Style::Inverted => Some(Box::new(style::Invert)),
        Style::Italic => Some(Box::new(style::Italic)),
    }
}