//! Interactive TUI composer for form3.
//!
//! Run with: `cargo run --example tui`
//!
//! Use arrow keys to navigate, Enter to select, 'q' to quit.

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use form3::ansi::{self, Attr, Color};
use form3::table::{Cell, Table, TableStyle};
use std::io::{self, Write};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    let mut selected_fg: Option<Color> = Some(Color::Red);
    let mut selected_attr: Option<Attr> = Some(Attr::Bold);
    let mut preview_text = "form3".to_string();

    loop {
        write!(stdout, "{}{}\r", ansi::clear_line(), ansi::move_up(8))?;

        let mut table = Table::new();
        table.set_header(vec![Cell::new("Control"), Cell::new("Value")]);
        table.add_row(vec![Cell::new("Text"), Cell::new(&preview_text)]);
        table.add_row(vec![
            Cell::new("Foreground"),
            Cell::new(&format!("{:?}", selected_fg)),
        ]);
        table.add_row(vec![
            Cell::new("Attribute"),
            Cell::new(&format!("{:?}", selected_attr)),
        ]);
        table.add_row(vec![
            Cell::new("Preview"),
            styled_preview(&preview_text, selected_fg, selected_attr).into(),
        ]);
        table.set_style(TableStyle::Rounded);

        writeln!(stdout, "{table}")?;
        writeln!(stdout, "Arrow keys: change values | Enter: cycle | q: quit")?;
        stdout.flush()?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Enter | KeyCode::Down => {
                    selected_fg = next_color(selected_fg);
                    selected_attr = next_attr(selected_attr);
                }
                KeyCode::Up => {
                    selected_fg = prev_color(selected_fg);
                    selected_attr = prev_attr(selected_attr);
                }
                KeyCode::Char(c) => preview_text.push(c),
                KeyCode::Backspace => {
                    preview_text.pop();
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}

fn styled_preview(text: &str, fg: Option<Color>, attr: Option<Attr>) -> String {
    use form3::ansi::{Attr, Color};
    use form3::compat::Colorize;
    let styled = text.into_styled();
    let styled = match fg {
        Some(Color::Black) => styled.black(),
        Some(Color::Red) => styled.red(),
        Some(Color::Green) => styled.green(),
        Some(Color::Yellow) => styled.yellow(),
        Some(Color::Blue) => styled.blue(),
        Some(Color::Magenta) => styled.magenta(),
        Some(Color::Cyan) => styled.cyan(),
        Some(Color::White) => styled.white(),
        None => styled,
    };
    let styled = match attr {
        Some(Attr::Bold) => styled.bold(),
        Some(Attr::Dim) => styled.dimmed(),
        Some(Attr::Italic) => styled.italic(),
        Some(Attr::Underline) => styled.underline(),
        Some(Attr::Reverse) => styled.reversed(),
        _ => styled,
    };
    styled.to_string()
}

fn next_color(color: Option<Color>) -> Option<Color> {
    let colors = [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::White,
    ];
    match color {
        None => Some(colors[0]),
        Some(c) => {
            let index = colors.iter().position(|x| *x == c).unwrap_or(0);
            Some(colors[(index + 1) % colors.len()])
        }
    }
}

fn prev_color(color: Option<Color>) -> Option<Color> {
    let colors = [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::White,
    ];
    match color {
        None => Some(colors[colors.len() - 1]),
        Some(c) => {
            let index = colors.iter().position(|x| *x == c).unwrap_or(0);
            Some(colors[(index + colors.len() - 1) % colors.len()])
        }
    }
}

fn next_attr(attr: Option<Attr>) -> Option<Attr> {
    let attrs = [
        Attr::Bold,
        Attr::Dim,
        Attr::Italic,
        Attr::Underline,
        Attr::Reverse,
    ];
    match attr {
        None => Some(attrs[0]),
        Some(a) => {
            let index = attrs.iter().position(|x| *x == a).unwrap_or(0);
            Some(attrs[(index + 1) % attrs.len()])
        }
    }
}

fn prev_attr(attr: Option<Attr>) -> Option<Attr> {
    let attrs = [
        Attr::Bold,
        Attr::Dim,
        Attr::Italic,
        Attr::Underline,
        Attr::Reverse,
    ];
    match attr {
        None => Some(attrs[attrs.len() - 1]),
        Some(a) => {
            let index = attrs.iter().position(|x| *x == a).unwrap_or(0);
            Some(attrs[(index + attrs.len() - 1) % attrs.len()])
        }
    }
}
