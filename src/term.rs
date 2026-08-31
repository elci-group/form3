//! Terminal capability detection: whether to emit color at all, and how
//! much. [`TermInfo::detect`] checks stdout's TTY status plus the
//! environment-wide signals — `NO_COLOR` (<https://no-color.org>),
//! `FORCE_COLOR`, `TERM=dumb`, and `COLORTERM`/`TERM` for 256-color and
//! truecolor support. A caller writing to a different or second stream
//! (e.g. stderr) should check `std::io::IsTerminal` on that stream itself
//! and construct its own [`TermInfo`]/[`ColorSupport`] — both are public,
//! plain-data types, not singletons — then feed the result in via
//! [`compat::StyledText::with_color_support`].

use std::env;
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

static FORCE_COLOR: AtomicBool = AtomicBool::new(false);
static FORCE_NO_COLOR: AtomicBool = AtomicBool::new(false);

/// Force color output on (`true`) or off (`false`) regardless of environment
/// or TTY detection. This restores the behavior that `deckhand` and
/// `ferret-cli` relied on before the term API refactor.
pub fn set_override(enabled: bool) {
    FORCE_COLOR.store(enabled, Ordering::SeqCst);
    FORCE_NO_COLOR.store(!enabled, Ordering::SeqCst);
}

/// Remove any override set by [`set_override`], returning color detection to
/// environment/TTY signals.
pub fn unset_override() {
    FORCE_COLOR.store(false, Ordering::SeqCst);
    FORCE_NO_COLOR.store(false, Ordering::SeqCst);
}

fn override_color_support() -> Option<ColorSupport> {
    if FORCE_NO_COLOR.load(Ordering::SeqCst) {
        return Some(ColorSupport::NoColor);
    }
    if FORCE_COLOR.load(Ordering::SeqCst) {
        return Some(ColorSupport::Truecolor);
    }
    None
}

/// How much color a caller has decided is safe to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorSupport {
    #[default]
    NoColor,
    Standard16,
    Color256,
    Truecolor,
}

impl ColorSupport {
    /// Parse color support purely from environment signals.
    ///
    /// This is useful for streams other than stdout: check `IsTerminal` on the
    /// stream, then call `ColorSupport::from_env(is_tty)`.
    pub fn from_env(is_tty: bool) -> Self {
        Self::detect_internal(
            is_tty,
            env::var("TERM").ok().as_deref(),
            env::var("COLORTERM").ok().as_deref(),
            env::var_os("FORCE_COLOR").is_some(),
            env::var_os("NO_COLOR").is_some(),
        )
    }

    /// Parse color support for any type implementing `IsTerminal`.
    pub fn from_stream(stream: &dyn IsTerminal) -> Self {
        Self::from_env(stream.is_terminal())
    }

    pub(crate) fn detect_internal(
        is_tty: bool,
        term: Option<&str>,
        colorterm: Option<&str>,
        forced: bool,
        no_color: bool,
    ) -> Self {
        if let Some(overridden) = override_color_support() {
            return overridden;
        }

        let dumb = term == Some("dumb");

        if no_color || dumb {
            return ColorSupport::NoColor;
        }

        if !is_tty && !forced {
            return ColorSupport::NoColor;
        }

        if colorterm == Some("truecolor") || colorterm == Some("24bit") {
            return ColorSupport::Truecolor;
        }

        if term.is_some_and(|t| t.contains("256color")) {
            return ColorSupport::Color256;
        }

        ColorSupport::Standard16
    }

    pub fn supports_color(&self) -> bool {
        self.color_support() != ColorSupport::NoColor
    }

    pub fn supports_256(&self) -> bool {
        matches!(
            self.color_support(),
            ColorSupport::Color256 | ColorSupport::Truecolor
        )
    }

    pub fn supports_truecolor(&self) -> bool {
        self.color_support() == ColorSupport::Truecolor
    }

    fn color_support(&self) -> ColorSupport {
        *self
    }
}

/// Environment- and TTY-derived color defaults.
#[derive(Debug, Clone, Copy)]
pub struct TermInfo {
    pub is_tty: bool,
    pub color_support: ColorSupport,
    pub term_program: Option<&'static str>,
}

impl TermInfo {
    /// Detect terminal capabilities from stdout's TTY status and the
    /// environment. `NO_COLOR`/`TERM=dumb` always force
    /// [`ColorSupport::NoColor`]; `FORCE_COLOR` re-enables color on a
    /// non-TTY stream (e.g. a pipe into `less -R`); a real TTY additionally
    /// checks `COLORTERM=truecolor` and a `*256color*` `TERM` for richer
    /// palettes.
    pub fn detect() -> Self {
        let is_tty = std::io::stdout().is_terminal();
        let term = env::var("TERM").ok();
        let colorterm = env::var("COLORTERM").ok();
        let term_program = env::var("TERM_PROGRAM").ok();

        Self {
            is_tty,
            color_support: ColorSupport::detect_internal(
                is_tty,
                term.as_deref(),
                colorterm.as_deref(),
                env::var_os("FORCE_COLOR").is_some(),
                env::var_os("NO_COLOR").is_some(),
            ),
            term_program: term_program.map(leak_str),
        }
    }

    pub fn supports_color(&self) -> bool {
        self.color_support.supports_color()
    }

    pub fn supports_256(&self) -> bool {
        self.color_support.supports_256()
    }

    pub fn supports_truecolor(&self) -> bool {
        self.color_support.supports_truecolor()
    }
}

fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_color_env_forces_no_color() {
        std::env::set_var("NO_COLOR", "1");
        assert_eq!(TermInfo::detect().color_support, ColorSupport::NoColor);
        std::env::remove_var("NO_COLOR");
    }

    #[test]
    fn dumb_term_forces_no_color() {
        assert_eq!(
            ColorSupport::detect_internal(true, Some("dumb"), None, false, false),
            ColorSupport::NoColor
        );
    }

    #[test]
    fn no_color_env_forces_no_color_internal() {
        assert_eq!(
            ColorSupport::detect_internal(true, Some("xterm"), None, false, true),
            ColorSupport::NoColor
        );
    }

    #[test]
    fn force_color_enables_standard16_on_non_tty() {
        assert_eq!(
            ColorSupport::detect_internal(false, Some("xterm"), None, true, false),
            ColorSupport::Standard16
        );
    }

    #[test]
    fn non_tty_without_force_color_is_no_color() {
        assert_eq!(
            ColorSupport::detect_internal(false, Some("xterm"), None, false, false),
            ColorSupport::NoColor
        );
    }

    #[test]
    fn colorterm_truecolor_detected() {
        assert_eq!(
            ColorSupport::detect_internal(true, Some("xterm"), Some("truecolor"), false, false),
            ColorSupport::Truecolor
        );
    }

    #[test]
    fn term_256color_detected() {
        assert_eq!(
            ColorSupport::detect_internal(true, Some("xterm-256color"), None, false, false),
            ColorSupport::Color256
        );
    }

    #[test]
    fn no_colorterm_falls_back_to_standard16() {
        assert_eq!(
            ColorSupport::detect_internal(true, Some("xterm"), None, false, false),
            ColorSupport::Standard16
        );
    }

    #[test]
    fn detect_is_tty_matches_stdout() {
        assert_eq!(TermInfo::detect().is_tty, std::io::stdout().is_terminal());
    }
}
