//! Raw SGR (Select Graphic Rendition) and OSC sequence construction. Callers
//! pick *whether* to emit these (see [`crate::term`]); this module only knows
//! how to spell them correctly.

/// One of the 8 standard terminal colors. Paired with a `bright` flag in
/// [`AnsiColor::Standard`] to reach all 16.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Color {
    fn base_offset(self) -> u8 {
        match self {
            Color::Black => 0,
            Color::Red => 1,
            Color::Green => 2,
            Color::Yellow => 3,
            Color::Blue => 4,
            Color::Magenta => 5,
            Color::Cyan => 6,
            Color::White => 7,
        }
    }
}

/// A color to render, from the 16-color standard palette, a 256-color index,
/// or a 24-bit RGB truecolor value (`38;2;r;g;b` / `48;2;r;g;b`) for brand
/// palettes that don't map cleanly onto the standard 16.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiColor {
    Standard(Color, bool),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// A single SGR text attribute (bold, underline, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attr {
    Bold,
    Dim,
    Italic,
    Underline,
    SlowBlink,
    RapidBlink,
    Reverse,
    Hidden,
    Strikethrough,
    DoubleUnderline,
    Overline,
}

/// Foreground-color escape sequence for `color`.
pub fn fg(color: &AnsiColor) -> String {
    match *color {
        AnsiColor::Standard(c, bright) => {
            let code = 30 + c.base_offset() + if bright { 60 } else { 0 };
            format!("\x1b[{code}m")
        }
        AnsiColor::Indexed(index) => format!("\x1b[38;5;{index}m"),
        AnsiColor::Rgb(r, g, b) => format!("\x1b[38;2;{r};{g};{b}m"),
    }
}

/// Background-color escape sequence for `color`.
pub fn bg(color: &AnsiColor) -> String {
    match *color {
        AnsiColor::Standard(c, bright) => {
            let code = 40 + c.base_offset() + if bright { 60 } else { 0 };
            format!("\x1b[{code}m")
        }
        AnsiColor::Indexed(index) => format!("\x1b[48;5;{index}m"),
        AnsiColor::Rgb(r, g, b) => format!("\x1b[48;2;{r};{g};{b}m"),
    }
}

/// Escape sequence for a single text attribute.
pub fn sgr(attr: Attr) -> String {
    let code = match attr {
        Attr::Bold => 1,
        Attr::Dim => 2,
        Attr::Italic => 3,
        Attr::Underline => 4,
        Attr::SlowBlink => 5,
        Attr::RapidBlink => 6,
        Attr::Reverse => 7,
        Attr::Hidden => 8,
        Attr::Strikethrough => 9,
        Attr::DoubleUnderline => 21,
        Attr::Overline => 53,
    };
    format!("\x1b[{code}m")
}

/// The sequence that clears all SGR state back to the terminal default.
pub fn reset() -> &'static str {
    "\x1b[0m"
}

/// An OSC 8 hyperlink sequence linking `text` to `url`.
///
/// If `url` is empty, emits a closing hyperlink sequence around `text`.
pub fn hyperlink(url: &str, text: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\")
}

/// Move the cursor up `n` lines.
pub fn move_up(n: u16) -> String {
    format!("\x1b[{n}A")
}

/// Clear the current line from the cursor to the end.
pub fn clear_line() -> &'static str {
    "\x1b[2K\r"
}

/// Hide the terminal cursor.
pub fn hide_cursor() -> &'static str {
    "\x1b[?25l"
}

/// Show the terminal cursor.
pub fn show_cursor() -> &'static str {
    "\x1b[?25h"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_fg_matches_classic_sgr_codes() {
        assert_eq!(fg(&AnsiColor::Standard(Color::Red, false)), "\x1b[31m");
        assert_eq!(fg(&AnsiColor::Standard(Color::Green, true)), "\x1b[92m");
    }

    #[test]
    fn indexed_fg_emits_256_color_sequence() {
        assert_eq!(fg(&AnsiColor::Indexed(196)), "\x1b[38;5;196m");
    }

    #[test]
    fn rgb_fg_emits_truecolor_sequence() {
        assert_eq!(fg(&AnsiColor::Rgb(176, 38, 255)), "\x1b[38;2;176;38;255m");
    }

    #[test]
    fn bg_uses_40_offset() {
        assert_eq!(bg(&AnsiColor::Standard(Color::Cyan, false)), "\x1b[46m");
    }

    #[test]
    fn indexed_bg_emits_256_color_sequence() {
        assert_eq!(bg(&AnsiColor::Indexed(82)), "\x1b[48;5;82m");
    }

    #[test]
    fn sgr_emits_expected_codes() {
        assert_eq!(sgr(Attr::Bold), "\x1b[1m");
        assert_eq!(sgr(Attr::DoubleUnderline), "\x1b[21m");
        assert_eq!(sgr(Attr::Overline), "\x1b[53m");
    }

    #[test]
    fn hyperlink_emits_osc_8_sequence() {
        assert_eq!(
            hyperlink("https://example.com", "click"),
            "\x1b]8;;https://example.com\x1b\\click\x1b]8;;\x1b\\"
        );
    }
}
