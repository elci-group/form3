//! Deterministic API surface for agent/model workflows.
//!
//! The agent module treats the API call loop as the primary interaction
//! surface. Models do not need to construct string-formatting adapters;
//! they emit typed [`Request`] values and receive rendered [`Response`]
//! strings that are ready to print.
//!
//! A CLI JSON-RPC wrapper is available via the `3form-agent` binary.

use crate::anim::{ProgressBar, ProgressStyle, Spinner};
use crate::ansi::{self, AnsiColor, Attr, Color};
use crate::compat::Colorize;
use crate::table::{Cell, CellAlignment, ColumnConstraint, RowStyle, Table, TableStyle, Width};
use serde::{Deserialize, Serialize};

/// Version of the agent API schema. Bumped on breaking changes.
pub const API_VERSION: &str = "1.0.0";

/// A typed request to the agent surface.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Render styled text.
    Style {
        text: String,
        #[serde(default)]
        fg: Option<String>,
        #[serde(default)]
        bg: Option<String>,
        #[serde(default)]
        attrs: Vec<String>,
    },
    /// Render a table.
    Table {
        #[serde(default)]
        header: Vec<String>,
        rows: Vec<Vec<String>>,
        #[serde(default)]
        style: Option<String>,
        #[serde(default)]
        alignments: Vec<String>,
        #[serde(default)]
        constraints: Vec<String>,
    },
    /// Render a spinner frame.
    Spinner {
        #[serde(default)]
        frames: Option<Vec<String>>,
        #[serde(default)]
        tick: Option<u64>,
    },
    /// Render a progress bar.
    Progress {
        length: u64,
        position: u64,
        #[serde(default)]
        template: Option<String>,
        #[serde(default)]
        style: Option<String>,
    },
    /// Reset/clear a line.
    Reset,
}

/// Response from the agent surface.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Response {
    pub version: String,
    pub output: String,
    pub error: Option<String>,
}

impl Response {
    fn ok(output: String) -> Self {
        Self {
            version: API_VERSION.to_string(),
            output,
            error: None,
        }
    }

    fn err(message: impl Into<String>) -> Self {
        Self {
            version: API_VERSION.to_string(),
            output: String::new(),
            error: Some(message.into()),
        }
    }
}

/// Stateless agent composer. Call [`Agent::compose`] in a loop.
#[derive(Debug, Clone)]
pub struct Agent {
    color_support: crate::term::ColorSupport,
}

impl Default for Agent {
    fn default() -> Self {
        Self {
            color_support: crate::term::ColorSupport::Truecolor,
        }
    }
}

impl Agent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_color_support(mut self, support: crate::term::ColorSupport) -> Self {
        self.color_support = support;
        self
    }

    pub fn compose(&self, request: Request) -> Response {
        match request {
            Request::Style {
                text,
                fg,
                bg,
                attrs,
            } => self.style(text, fg, bg, attrs),
            Request::Table {
                header,
                rows,
                style,
                alignments,
                constraints,
            } => self.table(header, rows, style, alignments, constraints),
            Request::Spinner { frames, tick } => self.spinner(frames, tick),
            Request::Progress {
                length,
                position,
                template,
                style,
            } => self.progress(length, position, template, style),
            Request::Reset => Response::ok(ansi::clear_line().to_string()),
        }
    }

    fn style(
        &self,
        text: String,
        fg: Option<String>,
        bg: Option<String>,
        attrs: Vec<String>,
    ) -> Response {
        let mut styled = text.into_styled().with_color_support(self.color_support);
        if let Some(color) = fg.and_then(parse_color) {
            styled = styled.set_fg(color);
        }
        if let Some(color) = bg.and_then(parse_color) {
            styled = styled.set_bg(color);
        }
        for attr in attrs {
            if let Some(attr) = parse_attr(&attr) {
                styled = styled.attr(attr);
            }
        }
        Response::ok(styled.to_string())
    }

    fn table(
        &self,
        header: Vec<String>,
        rows: Vec<Vec<String>>,
        style: Option<String>,
        alignments: Vec<String>,
        constraints: Vec<String>,
    ) -> Response {
        let mut table = Table::new();
        if let Some(style) = style.as_deref().and_then(parse_table_style) {
            table.set_style(style);
        }
        if !header.is_empty() {
            table.set_header(header.into_iter().map(Cell::new).collect::<Vec<_>>());
        }
        for (row_index, row) in rows.into_iter().enumerate() {
            table.add_row(row.into_iter().map(Cell::new).collect::<Vec<_>>());
            if row_index % 2 == 1 {
                table.set_row_style(row_index, RowStyle::new().dim());
            }
        }
        for (index, alignment) in alignments.iter().enumerate() {
            if let Some(column) = table.column_mut(index) {
                if let Some(align) = parse_alignment(alignment) {
                    column.set_cell_alignment(align);
                }
            }
        }
        let constraints: Vec<ColumnConstraint> = constraints
            .into_iter()
            .filter_map(|c| parse_constraint(&c))
            .collect();
        if !constraints.is_empty() {
            table.set_constraints(constraints);
        }
        Response::ok(table.to_string())
    }

    fn spinner(&self, frames: Option<Vec<String>>, tick: Option<u64>) -> Response {
        let tick = tick.unwrap_or(0);
        let frame = if let Some(frames) = frames {
            let frames: Vec<&'static str> = frames
                .into_iter()
                .map(|s| -> &'static str { Box::leak(s.into_boxed_str()) })
                .collect();
            if frames.is_empty() {
                return Response::err("spinner frames must not be empty");
            }
            Spinner::new(&*Box::leak(frames.into_boxed_slice())).frame(tick)
        } else {
            Spinner::default().frame(tick)
        };
        Response::ok(frame.to_string())
    }

    fn progress(
        &self,
        length: u64,
        position: u64,
        template: Option<String>,
        style: Option<String>,
    ) -> Response {
        let mut bar = ProgressBar::new(length);
        bar.set_position(position);
        if let Some(template) = template {
            bar.set_template(template);
        }
        if let Some(style) = style.as_deref() {
            match style {
                "block" => ProgressStyle::Block.apply(&mut bar),
                "line" => ProgressStyle::Line.apply(&mut bar),
                "ascii" => ProgressStyle::Ascii.apply(&mut bar),
                _ => ProgressStyle::Default.apply(&mut bar),
            }
        }
        Response::ok(bar.to_string())
    }
}

fn parse_color(value: String) -> Option<AnsiColor> {
    if value.starts_with('#') && value.len() == 7 {
        let r = u8::from_str_radix(&value[1..3], 16).ok()?;
        let g = u8::from_str_radix(&value[3..5], 16).ok()?;
        let b = u8::from_str_radix(&value[5..7], 16).ok()?;
        return Some(AnsiColor::Rgb(r, g, b));
    }
    if value.starts_with("rgb(") && value.ends_with(')') {
        let inner = &value[4..value.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() == 3 {
            let r = parts[0].parse().ok()?;
            let g = parts[1].parse().ok()?;
            let b = parts[2].parse().ok()?;
            return Some(AnsiColor::Rgb(r, g, b));
        }
    }
    if let Ok(index) = value.parse::<u8>() {
        return Some(AnsiColor::Indexed(index));
    }
    parse_named_color(&value).map(|(color, bright)| AnsiColor::Standard(color, bright))
}

fn parse_named_color(name: &str) -> Option<(Color, bool)> {
    let (name, bright) = if let Some(rest) = name.strip_prefix("bright_") {
        (rest, true)
    } else {
        (name, false)
    };
    let color = match name {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "purple" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        _ => return None,
    };
    Some((color, bright))
}

fn parse_attr(name: &str) -> Option<Attr> {
    match name {
        "bold" => Some(Attr::Bold),
        "dim" => Some(Attr::Dim),
        "italic" => Some(Attr::Italic),
        "underline" => Some(Attr::Underline),
        "double_underline" => Some(Attr::DoubleUnderline),
        "overline" => Some(Attr::Overline),
        "slow_blink" => Some(Attr::SlowBlink),
        "rapid_blink" => Some(Attr::RapidBlink),
        "reverse" => Some(Attr::Reverse),
        "hidden" => Some(Attr::Hidden),
        "strikethrough" => Some(Attr::Strikethrough),
        _ => None,
    }
}

fn parse_table_style(name: &str) -> Option<TableStyle> {
    match name {
        "rounded" => Some(TableStyle::Rounded),
        "square" => Some(TableStyle::Square),
        "ascii" => Some(TableStyle::Ascii),
        "markdown" => Some(TableStyle::Markdown),
        "psql" => Some(TableStyle::Psql),
        "heavy" => Some(TableStyle::Heavy),
        _ => None,
    }
}

fn parse_alignment(name: &str) -> Option<CellAlignment> {
    match name {
        "left" => Some(CellAlignment::Left),
        "center" => Some(CellAlignment::Center),
        "right" => Some(CellAlignment::Right),
        _ => None,
    }
}

fn parse_constraint(value: &str) -> Option<ColumnConstraint> {
    if let Some(width) = value.strip_prefix("max:") {
        let width = width.parse().ok()?;
        return Some(ColumnConstraint::MaxWidth(width));
    }
    if let Some(width) = value.strip_prefix("min:") {
        let width = width.parse().ok()?;
        return Some(ColumnConstraint::MinWidth(width));
    }
    if let Some(pct) = value.strip_prefix("pct:") {
        let pct = pct.parse().ok()?;
        return Some(ColumnConstraint::Percentage(pct));
    }
    if let Some(width) = value.strip_prefix("fixed:") {
        let width = width.parse().ok()?;
        return Some(ColumnConstraint::LowerBoundary(Width::Fixed(width)));
    }
    None
}

/// Read a single JSON-RPC request object from `input` and write the response
/// to `output`.
///
/// This function is used by the `3form-agent` binary. It expects one request
/// per line (newline-delimited JSON).
pub fn handle_json_request(input: &str, output: &mut impl std::fmt::Write) -> Result<(), String> {
    let request: Request = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let response = Agent::new().compose(request);
    let json = serde_json::to_string(&response).map_err(|e| e.to_string())?;
    writeln!(output, "{json}").map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::BRAILLE;

    #[test]
    fn style_request_renders_ansi() {
        let agent = Agent::new();
        let response = agent.compose(Request::Style {
            text: "hello".to_string(),
            fg: Some("red".to_string()),
            bg: None,
            attrs: vec!["bold".to_string()],
        });
        assert!(response.output.contains("\x1b[31m"));
        assert!(response.output.contains("\x1b[1m"));
        assert!(response.error.is_none());
    }

    #[test]
    fn table_request_renders_grid() {
        let agent = Agent::new();
        let response = agent.compose(Request::Table {
            header: vec!["A".to_string()],
            rows: vec![vec!["1".to_string()]],
            style: Some("ascii".to_string()),
            alignments: vec![],
            constraints: vec![],
        });
        assert!(response.output.contains('+'));
    }

    #[test]
    fn spinner_request_returns_frame() {
        let agent = Agent::new();
        let response = agent.compose(Request::Spinner {
            frames: None,
            tick: Some(0),
        });
        assert_eq!(response.output, BRAILLE[0]);
    }

    #[test]
    fn progress_request_renders_bar() {
        let agent = Agent::new();
        let response = agent.compose(Request::Progress {
            length: 10,
            position: 5,
            template: None,
            style: None,
        });
        assert!(response.output.contains('█'));
        assert!(response.output.contains('░'));
    }

    #[test]
    fn json_round_trip() {
        let request = Request::Style {
            text: "hi".to_string(),
            fg: Some("#ff0000".to_string()),
            bg: None,
            attrs: vec![],
        };
        let json = serde_json::to_string(&request).unwrap();
        let mut output = String::new();
        handle_json_request(&json, &mut output).expect("handle_json_request failed");
        assert!(output.contains("hi"));
    }
}
