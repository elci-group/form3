//! Frame-based animation primitives. `form3` has no runtime of its own —
//! a [`Spinner`] just maps a tick counter to a frame; the caller owns the
//! timing loop (a `tokio::time::interval`, a plain `std::thread::sleep`,
//! whatever fits) and decides when/where to print.
//!
//! For stateful rendering helpers such as progress bars, see [`ProgressBar`]
//! and [`RenderSurface`].

use crate::term::ColorSupport;
use std::fmt;
use std::io::IsTerminal;
use std::time::Duration;

/// A braille-dot spin cycle. Renders cleanly in any UTF-8 terminal without
/// needing a wide font fallback.
pub const BRAILLE: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// A plain ASCII spin cycle for terminals/logs that can't render braille.
pub const ASCII: [&str; 4] = ["|", "/", "-", "\\"];

/// Dots bouncing left-to-right.
pub const DOTS: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Smaller dots variant.
pub const DOTS2: [&str; 6] = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟"];

/// Even smaller dots variant.
pub const DOTS3: [&str; 6] = ["⠋", "⠙", "⠚", "⠞", "⠖", "⠦"];

/// A horizontal line scrolling through the frame.
pub const LINE: [&str; 6] = ["-", "\\", "|", "/", "-", "\\"];

/// A spinning star.
pub const STAR: [&str; 4] = ["✶", "✸", "✹", "✺"];

/// A pulsing star.
pub const STAR2: [&str; 4] = ["+", "x", "*", "-"];

/// A flipping card.
pub const FLIP: [&str; 6] = ["_", "_", "_", "-", "`", "`"];

/// A bouncing ball.
pub const BOUNCE: [&str; 6] = [
    "( ●    )",
    "(  ●   )",
    "(   ●  )",
    "(    ● )",
    "(     ●)",
    "(    ● )",
];

/// Pong-style animation.
pub const PONG: [&str; 8] = [
    "( ●    )",
    "(  ●   )",
    "(   ●  )",
    "(    ● )",
    "(     ●)",
    "(    ● )",
    "(   ●  )",
    "(  ●   )",
];

/// A box that bounces around.
pub const BOX_BOUNCE: [&str; 4] = ["▖", "▘", "▝", "▗"];

/// Recommended interval between frames for a given frame set, in milliseconds.
pub fn interval_ms(frames: &[&str]) -> u64 {
    match frames.len() {
        0 => 100,
        1..=4 => 120,
        5..=8 => 100,
        _ => 80,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Spinner {
    frames: &'static [&'static str],
}

impl Default for Spinner {
    fn default() -> Self {
        Self { frames: &BRAILLE }
    }
}

impl Spinner {
    pub fn new(frames: &'static [&'static str]) -> Self {
        assert!(!frames.is_empty(), "Spinner needs at least one frame");
        Self { frames }
    }

    /// The frame for `tick`. Wraps around, so any monotonically
    /// increasing counter works.
    pub fn frame(&self, tick: u64) -> &'static str {
        self.frames[(tick as usize) % self.frames.len()]
    }

    /// The frame for a given elapsed duration, using the spinner's recommended
    /// frame interval.
    pub fn frame_for(&self, elapsed: Duration) -> &'static str {
        let interval = Duration::from_millis(interval_ms(self.frames));
        if interval.is_zero() {
            return self.frames[0];
        }
        let tick = elapsed.as_millis() as u64 / interval.as_millis() as u64;
        self.frame(tick)
    }

    /// Recommended update interval for this spinner.
    pub fn interval(&self) -> Duration {
        Duration::from_millis(interval_ms(self.frames))
    }
}

/// A pluggable output surface for animation and progress rendering.
///
/// Implementations are provided for `std::io::Stdout` and `std::io::Stderr`;
/// tests can provide a custom surface that records written bytes.
pub trait RenderSurface {
    fn write(&mut self, text: &str);
    fn flush(&mut self);
}

impl RenderSurface for std::io::Stdout {
    fn write(&mut self, text: &str) {
        let _ = std::io::Write::write_all(self, text.as_bytes());
    }
    fn flush(&mut self) {
        let _ = std::io::Write::flush(self);
    }
}

impl RenderSurface for std::io::Stderr {
    fn write(&mut self, text: &str) {
        let _ = std::io::Write::write_all(self, text.as_bytes());
    }
    fn flush(&mut self) {
        let _ = std::io::Write::flush(self);
    }
}

/// A recording surface for tests and model workflows.
#[derive(Debug, Clone, Default)]
pub struct BufferSurface {
    pub lines: Vec<String>,
}

impl BufferSurface {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content(&self) -> String {
        self.lines.join("\n")
    }
}

impl RenderSurface for BufferSurface {
    fn write(&mut self, text: &str) {
        if let Some(last) = self.lines.last_mut() {
            last.push_str(text);
        } else {
            self.lines.push(text.to_string());
        }
    }

    fn flush(&mut self) {}
}

/// A simple progress bar driven by the caller.
///
/// # Example
///
/// ```rust
/// use form3::anim::{ProgressBar, ProgressStyle};
/// let mut bar = ProgressBar::new(100);
/// ProgressStyle::Default.apply(&mut bar);
/// bar.set_position(42);
/// let rendered = bar.to_string();
/// ```
#[derive(Debug, Clone)]
pub struct ProgressBar {
    length: u64,
    position: u64,
    template: String,
    bar_width: usize,
    filled: char,
    empty: char,
}

impl ProgressBar {
    pub fn new(length: u64) -> Self {
        Self {
            length,
            position: 0,
            template: "{bar} {percent}%".to_string(),
            bar_width: 40,
            filled: '█',
            empty: '░',
        }
    }

    pub fn set_position(&mut self, position: u64) -> &mut Self {
        self.position = position.min(self.length);
        self
    }

    pub fn inc(&mut self, delta: u64) -> &mut Self {
        self.position = (self.position + delta).min(self.length);
        self
    }

    pub fn set_template(&mut self, template: impl Into<String>) -> &mut Self {
        self.template = template.into();
        self
    }

    pub fn set_bar_style(&mut self, filled: char, empty: char, width: usize) -> &mut Self {
        self.filled = filled;
        self.empty = empty;
        self.bar_width = width;
        self
    }

    fn percent(&self) -> u64 {
        self.position
            .checked_mul(100)
            .and_then(|num| num.checked_div(self.length.max(1)))
            .unwrap_or(100)
    }

    fn render_bar(&self) -> String {
        if self.bar_width == 0 {
            return String::new();
        }
        let filled = if self.length == 0 {
            self.bar_width
        } else {
            ((self.position as usize * self.bar_width) / self.length as usize).min(self.bar_width)
        };
        let mut bar = String::with_capacity(self.bar_width);
        for _ in 0..filled {
            bar.push(self.filled);
        }
        for _ in filled..self.bar_width {
            bar.push(self.empty);
        }
        bar
    }

    fn elapsed(&self) -> Duration {
        Duration::default()
    }

    fn eta(&self) -> Duration {
        Duration::default()
    }
}

impl fmt::Display for ProgressBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = self.template.clone();
        output = output.replace("{bar}", &self.render_bar());
        output = output.replace("{pos}", &self.position.to_string());
        output = output.replace("{len}", &self.length.to_string());
        output = output.replace("{percent}", &format!("{:3}", self.percent()));
        output = output.replace("{elapsed}", &format!("{:?}", self.elapsed()));
        output = output.replace("{eta}", &format!("{:?}", self.eta()));
        write!(f, "{output}")
    }
}

/// Predefined progress-bar styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStyle {
    Default,
    Block,
    Line,
    Ascii,
}

impl ProgressStyle {
    pub fn apply(&self, bar: &mut ProgressBar) {
        match self {
            ProgressStyle::Default => {
                bar.set_bar_style('█', '░', 40);
            }
            ProgressStyle::Block => {
                bar.set_bar_style('█', '▒', 40);
            }
            ProgressStyle::Line => {
                bar.set_bar_style('─', '·', 40);
            }
            ProgressStyle::Ascii => {
                bar.set_bar_style('=', '-', 40);
            }
        }
    }
}

/// Renders multiple progress bars concurrently to a [`RenderSurface`].
#[derive(Debug)]
pub struct MultiProgress<S: RenderSurface> {
    surface: S,
    bars: Vec<ProgressBar>,
}

impl<S: RenderSurface> MultiProgress<S> {
    pub fn new(surface: S) -> Self {
        Self {
            surface,
            bars: Vec::new(),
        }
    }

    pub fn add(&mut self, bar: ProgressBar) -> usize {
        let index = self.bars.len();
        self.bars.push(bar);
        index
    }

    pub fn bar_mut(&mut self, index: usize) -> Option<&mut ProgressBar> {
        self.bars.get_mut(index)
    }

    /// Redraw all bars, moving the cursor up to overwrite previous output.
    pub fn draw(&mut self) {
        for _ in 0..self.bars.len() {
            self.surface.write(crate::ansi::clear_line());
            self.surface.write(&crate::ansi::move_up(1));
        }
        for bar in &self.bars {
            self.surface.write(&bar.to_string());
            self.surface.write("\n");
        }
        self.surface.flush();
    }
}

/// Deprecated animation helpers kept as minimal stubs so downstream crates
/// (`ferret-cli`, `chakra`, `isopod`) keep compiling. They render stable,
/// color-aware but otherwise no-op output rather than the original 3form
/// animation effects.
#[derive(Debug, Clone, Copy)]
pub struct Aura {
    start: (u8, u8, u8),
    mid: (u8, u8, u8),
    end: (u8, u8, u8),
}

impl Aura {
    pub fn new(start: (u8, u8, u8), mid: (u8, u8, u8), end: (u8, u8, u8)) -> Self {
        Self { start, mid, end }
    }

    pub fn mystic() -> Self {
        Self::new((92, 48, 25), (196, 126, 90), (255, 230, 211))
    }

    pub fn reveal(&self, text: &str, _progress: f32, _phase: f32, _support: ColorSupport) -> String {
        text.to_string()
    }

    pub fn paint(&self, text: &str, progress: f32, support: ColorSupport) -> String {
        if support == ColorSupport::NoColor {
            text.to_string()
        } else {
            // Stable truecolor gradient across the three configured colors.
            let t = progress.clamp(0.0, 1.0);
            let (a, b, t_local) = if t <= 0.5 {
                (self.start, self.mid, t * 2.0)
            } else {
                (self.mid, self.end, (t - 0.5) * 2.0)
            };
            let (r1, g1, b1) = a;
            let (r2, g2, b2) = b;
            let r = (r1 as f32 * (1.0 - t_local) + r2 as f32 * t_local) as u8;
            let g = (g1 as f32 * (1.0 - t_local) + g2 as f32 * t_local) as u8;
            let b = (b1 as f32 * (1.0 - t_local) + b2 as f32 * t_local) as u8;
            format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m")
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GrowthTrack {
    width: usize,
    start: char,
    empty: char,
    fill: char,
    end: char,
}

impl GrowthTrack {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            start: '·',
            empty: '·',
            fill: '✦',
            end: '›',
        }
    }

    pub fn with_glyphs(mut self, start: char, empty: char, fill: char, end: char) -> Self {
        self.start = start;
        self.empty = empty;
        self.fill = fill;
        self.end = end;
        self
    }

    /// Render the grown portion of the track. The length increases with
    /// `progress`, so callers can animate cells appearing without pre-printing
    /// future positions.
    pub fn render(&self, progress: f32) -> String {
        let filled = (progress * self.width as f32)
            .ceil()
            .max(0.0)
            .min(self.width as f32) as usize;
        let mut out = String::new();
        out.push(self.start);
        for _ in 0..filled {
            out.push(self.fill);
        }
        if progress >= 1.0 {
            out.push(self.end);
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Machine,
    Ui,
}

impl Mode {
    pub fn auto() -> Self {
        if std::io::stdout().is_terminal() {
            Mode::Ui
        } else {
            Mode::Machine
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IsopodCrawl {
    width: usize,
}

impl IsopodCrawl {
    pub fn new(width: usize) -> Self {
        Self { width }
    }

    pub fn render(&self, progress: f32, _tick: u32) -> String {
        let pos = (progress * self.width as f32)
            .round()
            .max(0.0)
            .min(self.width as f32) as usize;
        format!("{}🐛{}", "·".repeat(pos), "·".repeat(self.width.saturating_sub(pos)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_wrap_around() {
        let spinner = Spinner::default();
        assert_eq!(spinner.frame(0), BRAILLE[0]);
        assert_eq!(spinner.frame(BRAILLE.len() as u64), BRAILLE[0]);
    }

    #[test]
    fn custom_frame_set() {
        let spinner = Spinner::new(&ASCII);
        assert_eq!(spinner.frame(2), "-");
    }

    #[test]
    fn frame_for_duration() {
        let spinner = Spinner::new(&ASCII);
        assert_eq!(spinner.frame_for(Duration::from_millis(0)), "|");
        assert_eq!(
            spinner.frame_for(Duration::from_millis(
                spinner.interval().as_millis() as u64 * 2
            )),
            "-"
        );
    }

    #[test]
    fn progress_bar_renders_template() {
        let mut bar = ProgressBar::new(100);
        bar.set_position(50);
        let rendered = bar.to_string();
        assert!(rendered.contains("50%"));
        assert!(rendered.contains('█'));
        assert!(rendered.contains('░'));
    }

    #[test]
    fn progress_style_applies() {
        let mut bar = ProgressBar::new(10);
        ProgressStyle::Ascii.apply(&mut bar);
        bar.set_position(5);
        let rendered = bar.to_string();
        assert!(rendered.contains('='));
        assert!(rendered.contains('-'));
    }
}
