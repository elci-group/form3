// Copyright 2012-2025 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unicode display-width calculation for terminal layout.
//!
//! The core tables and algorithm are vendored from `unicode-width` v0.2.2
//! (MIT/Apache-2.0) and kept in-tree so that `form3` remains dependency-free.
//! This module strips ANSI escape sequences before measuring width, so callers
//! can pass already-styled text directly.

#![allow(dead_code)]

#[path = "width_tables.rs"]
mod width_tables;

mod private {
    pub trait Sealed {}
    impl Sealed for char {}
    impl Sealed for str {}
}

/// Methods for determining displayed width of Unicode characters.
pub(crate) trait UnicodeWidthChar: private::Sealed {
    fn width(self) -> Option<usize>;

    #[cfg(feature = "cjk")]
    fn width_cjk(self) -> Option<usize>;
}

impl UnicodeWidthChar for char {
    #[inline]
    fn width(self) -> Option<usize> {
        width_tables::single_char_width(self)
    }

    #[cfg(feature = "cjk")]
    #[inline]
    fn width_cjk(self) -> Option<usize> {
        width_tables::single_char_width_cjk(self)
    }
}

/// Methods for determining displayed width of Unicode strings.
pub(crate) trait UnicodeWidthStr: private::Sealed {
    fn width(&self) -> usize;

    #[cfg(feature = "cjk")]
    fn width_cjk(&self) -> usize;
}

impl UnicodeWidthStr for str {
    #[inline]
    fn width(&self) -> usize {
        width_tables::str_width(self)
    }

    #[cfg(feature = "cjk")]
    #[inline]
    fn width_cjk(&self) -> usize {
        width_tables::str_width_cjk(self)
    }
}

/// Display width of `text` in terminal columns, ignoring ANSI SGR sequences.
///
/// This is the function used by the table renderer; it first strips any
/// `\x1b[...m` style escape sequences, then applies the Unicode width rules
/// vendored from `unicode-width`.
pub fn display_width(text: &str) -> usize {
    let stripped = strip_ansi(text);
    stripped.width()
}

/// CJK-aware display width. Treats ambiguous-width characters as 2 columns.
#[cfg(feature = "cjk")]
pub fn display_width_cjk(text: &str) -> usize {
    let stripped = strip_ansi(text);
    stripped.width_cjk()
}

/// Returns a heap-allocated string with ANSI escape sequences removed.
///
/// Only CSI sequences (`\x1b[` ... `@`..`~`) are stripped, which is sufficient
/// for SGR codes and other terminal styling sequences emitted by `form3`.
fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
            continue;
        }
        result.push(ch);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_width() {
        assert_eq!(display_width("hello"), 5);
    }

    #[test]
    fn cjk_width() {
        assert_eq!(display_width("你好"), 4);
    }

    #[test]
    fn combining_width_zero() {
        assert_eq!(display_width("e\u{0301}"), 1);
    }

    #[test]
    fn ansi_escapes_ignored() {
        assert_eq!(display_width("\x1b[31mred\x1b[0m"), 3);
    }

    #[test]
    fn emoji_width() {
        assert_eq!(display_width("🎉"), 2);
    }
}
