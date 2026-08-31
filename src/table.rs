//! Dependency-free Unicode table rendering with ANSI-aware width handling.

use crate::ansi::{self, AnsiColor, Attr, Color};
use crate::width::display_width;
use std::fmt;

/// A cell-level text attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribute {
    Bold,
    Dim,
    Italic,
    Underline,
    DoubleUnderline,
    Overline,
    SlowBlink,
    RapidBlink,
    Reverse,
    Hidden,
    Strikethrough,
}

impl From<Attribute> for Attr {
    fn from(attribute: Attribute) -> Self {
        match attribute {
            Attribute::Bold => Attr::Bold,
            Attribute::Dim => Attr::Dim,
            Attribute::Italic => Attr::Italic,
            Attribute::Underline => Attr::Underline,
            Attribute::DoubleUnderline => Attr::DoubleUnderline,
            Attribute::Overline => Attr::Overline,
            Attribute::SlowBlink => Attr::SlowBlink,
            Attribute::RapidBlink => Attr::RapidBlink,
            Attribute::Reverse => Attr::Reverse,
            Attribute::Hidden => Attr::Hidden,
            Attribute::Strikethrough => Attr::Strikethrough,
        }
    }
}

/// Border style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableStyle {
    /// Rounded corners, double-line header rule, rules between every row.
    #[default]
    Rounded,
    /// Square corners, single rule after the header, no rules between data rows.
    Square,
    /// Plain ASCII `+-|`, with a rule after the header and after every data row.
    Ascii,
    /// Markdown-compatible pipe table with no outer borders.
    Markdown,
    /// PostgreSQL-style ASCII table with outer borders and header separator.
    Psql,
    /// Heavy-lined Unicode borders.
    Heavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellAlignment {
    #[default]
    Left,
    Center,
    Right,
}

/// A single table cell. Supports foreground/background colors and text attributes.
#[derive(Debug, Clone)]
pub struct Cell {
    text: String,
    fg: Option<Color>,
    bg: Option<Color>,
    attrs: Vec<Attribute>,
}

impl Cell {
    pub fn new(value: impl fmt::Display) -> Self {
        Self {
            text: value.to_string(),
            fg: None,
            bg: None,
            attrs: Vec::new(),
        }
    }

    pub fn add_attribute(mut self, attribute: Attribute) -> Self {
        self.attrs.push(attribute);
        self
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    fn render(&self) -> String {
        let mut codes = String::new();
        if let Some(color) = self.fg {
            codes.push_str(&ansi::fg(&AnsiColor::Standard(color, false)));
        }
        if let Some(color) = self.bg {
            codes.push_str(&ansi::bg(&AnsiColor::Standard(color, false)));
        }
        for attribute in &self.attrs {
            codes.push_str(&ansi::sgr((*attribute).into()));
        }
        if codes.is_empty() {
            self.text.clone()
        } else {
            format!("{codes}{}{}", self.text, ansi::reset())
        }
    }
}

impl<T: fmt::Display> From<T> for Cell {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

/// Width specification for column constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    /// Use the natural width of the content.
    Auto,
    /// Fixed width in columns.
    Fixed(u16),
    /// At least this many columns.
    Min(u16),
    /// At most this many columns.
    Max(u16),
}

/// Constraints on a column's width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnConstraint {
    LowerBoundary(Width),
    UpperBoundary(Width),
    MinWidth(u16),
    MaxWidth(u16),
    Percentage(u16),
}

/// How table content should be arranged horizontally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentArrangement {
    /// Use fixed widths as given by constraints.
    #[default]
    Fixed,
    /// Size columns to fit their content, then scale to terminal width.
    Dynamic,
    /// Hide overflow content rather than wrap or expand.
    Hidden,
}

/// Table-level style modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifier {
    round_corners: bool,
}

impl Modifier {
    pub const UTF8_ROUND_CORNERS: Self = Self {
        round_corners: true,
    };
}

pub mod modifiers {
    pub use super::Modifier;
}

/// Style applied to an entire row.
#[derive(Debug, Clone, Default)]
pub struct RowStyle {
    fg: Option<Color>,
    bg: Option<Color>,
    attrs: Vec<Attribute>,
}

impl RowStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn add_attribute(mut self, attribute: Attribute) -> Self {
        self.attrs.push(attribute);
        self
    }

    pub fn bold(self) -> Self {
        self.add_attribute(Attribute::Bold)
    }

    pub fn dim(self) -> Self {
        self.add_attribute(Attribute::Dim)
    }

    pub fn italic(self) -> Self {
        self.add_attribute(Attribute::Italic)
    }

    pub fn underline(self) -> Self {
        self.add_attribute(Attribute::Underline)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Column {
    alignment: CellAlignment,
    constraint: Option<ColumnConstraint>,
}

impl Column {
    pub fn set_cell_alignment(&mut self, alignment: CellAlignment) -> &mut Self {
        self.alignment = alignment;
        self
    }

    pub fn set_constraint(&mut self, constraint: ColumnConstraint) -> &mut Self {
        self.constraint = Some(constraint);
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct Table {
    header: Vec<Cell>,
    rows: Vec<Vec<Cell>>,
    columns: Vec<Column>,
    row_styles: Vec<RowStyle>,
    style: TableStyle,
    arrangement: ContentArrangement,
    modifier: Modifier,
}

impl Table {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_style(&mut self, style: TableStyle) -> &mut Self {
        self.style = style;
        self
    }

    pub fn set_header<T: Into<Cell>>(&mut self, cells: Vec<T>) -> &mut Self {
        self.header = cells.into_iter().map(Into::into).collect();
        self.ensure_columns();
        self
    }

    pub fn add_row<T: Into<Cell>>(&mut self, cells: Vec<T>) -> &mut Self {
        self.rows.push(cells.into_iter().map(Into::into).collect());
        self.row_styles.push(RowStyle::default());
        self.ensure_columns();
        self
    }

    pub fn set_row_style(&mut self, index: usize, style: RowStyle) -> &mut Self {
        if index < self.row_styles.len() {
            self.row_styles[index] = style;
        }
        self
    }

    pub fn apply_modifier(&mut self, modifier: Modifier) -> &mut Self {
        self.modifier = modifier;
        self
    }

    pub fn set_content_arrangement(&mut self, arrangement: ContentArrangement) -> &mut Self {
        self.arrangement = arrangement;
        self
    }

    pub fn set_constraints(&mut self, constraints: Vec<ColumnConstraint>) -> &mut Self {
        self.ensure_columns();
        for (index, constraint) in constraints.into_iter().enumerate() {
            if let Some(column) = self.columns.get_mut(index) {
                column.constraint = Some(constraint);
            }
        }
        self
    }

    pub fn column_mut(&mut self, index: usize) -> Option<&mut Column> {
        self.ensure_columns();
        self.columns.get_mut(index)
    }

    fn ensure_columns(&mut self) {
        let count = self
            .header
            .len()
            .max(self.rows.iter().map(Vec::len).max().unwrap_or(0));
        self.columns.resize_with(count, Column::default);
        self.row_styles
            .resize_with(self.rows.len(), RowStyle::default);
    }

    fn natural_widths(&self) -> Vec<usize> {
        (0..self.columns.len())
            .map(|index| {
                let header = self
                    .header
                    .get(index)
                    .map_or(0, |cell| display_width(&cell.text));
                let rows = self
                    .rows
                    .iter()
                    .filter_map(|row| row.get(index))
                    .map(|cell| display_width(&cell.text))
                    .max()
                    .unwrap_or(0);
                header.max(rows)
            })
            .collect()
    }

    fn apply_constraints(&self, natural: &[usize]) -> Vec<usize> {
        let total_width = terminal_width();
        let mut widths = natural.to_vec();

        for (index, column) in self.columns.iter().enumerate() {
            if index >= widths.len() {
                break;
            }
            if let Some(constraint) = column.constraint {
                let width = &mut widths[index];
                match constraint {
                    ColumnConstraint::LowerBoundary(Width::Fixed(w))
                    | ColumnConstraint::LowerBoundary(Width::Min(w))
                    | ColumnConstraint::MinWidth(w) => {
                        *width = (*width).max(w as usize);
                    }
                    ColumnConstraint::UpperBoundary(Width::Fixed(w))
                    | ColumnConstraint::UpperBoundary(Width::Max(w))
                    | ColumnConstraint::MaxWidth(w) => {
                        *width = (*width).min(w as usize);
                    }
                    ColumnConstraint::Percentage(p) => {
                        let target = (total_width * p as usize) / 100;
                        *width = (*width).min(target);
                    }
                    _ => {}
                }
            }
        }

        if self.arrangement == ContentArrangement::Dynamic {
            let content_total: usize = widths.iter().sum();
            let available = total_width.saturating_sub(widths.len() * 3 + 1);
            if content_total > 0 && content_total < available {
                let scale = available as f64 / content_total as f64;
                for width in &mut widths {
                    *width = ((*width as f64 * scale) as usize).max(1);
                }
            }
        }

        widths
    }

    fn widths(&self) -> Vec<usize> {
        let natural = self.natural_widths();
        self.apply_constraints(&natural)
    }
}

impl fmt::Display for Table {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let widths = self.widths();
        if widths.is_empty() {
            return Ok(());
        }

        let style = self.style;
        let round = self.modifier.round_corners || matches!(style, TableStyle::Rounded);

        match style {
            TableStyle::Rounded | TableStyle::Square | TableStyle::Heavy => {
                let chars = if round {
                    ('╭', '┬', '╮', '─', '├', '┼', '┤', '╰', '┴', '╯', '│', '═')
                } else if matches!(style, TableStyle::Heavy) {
                    ('┏', '┳', '┓', '━', '┣', '╋', '┫', '┗', '┻', '┛', '┃', '━')
                } else {
                    ('┌', '┬', '┐', '─', '├', '┼', '┤', '└', '┴', '┘', '│', '─')
                };
                let row_separators = !matches!(style, TableStyle::Square);
                render_boxed(formatter, self, &widths, chars, row_separators)
            }
            TableStyle::Ascii | TableStyle::Psql => {
                let chars = if matches!(style, TableStyle::Psql) {
                    ('┌', '┬', '┐', '─', '├', '┼', '┤', '└', '┴', '┘', '│', '─')
                } else {
                    ('+', '+', '+', '-', '+', '+', '+', '+', '+', '+', '|', '-')
                };
                let row_separators = !matches!(style, TableStyle::Psql);
                render_boxed(formatter, self, &widths, chars, row_separators)
            }
            TableStyle::Markdown => render_markdown(formatter, self, &widths),
        }
    }
}

fn render_boxed(
    f: &mut fmt::Formatter<'_>,
    table: &Table,
    widths: &[usize],
    chars: (
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
        char,
    ),
    row_separators: bool,
) -> fmt::Result {
    let (tl, tj, tr, h, ml, mj, mr, bl, bj, br, v, header_h) = chars;
    border(f, tl, tj, tr, h, widths)?;
    if !table.header.is_empty() {
        row(f, &table.header, &table.columns, widths, v, None)?;
        border(f, ml, mj, mr, header_h, widths)?;
    }
    for (index, cells) in table.rows.iter().enumerate() {
        let style = table.row_styles.get(index).cloned();
        row(f, cells, &table.columns, widths, v, style.as_ref())?;
        if row_separators && index + 1 < table.rows.len() {
            border(f, ml, mj, mr, h, widths)?;
        }
    }
    border(f, bl, bj, br, h, widths)
}

fn render_markdown(f: &mut fmt::Formatter<'_>, table: &Table, widths: &[usize]) -> fmt::Result {
    if !table.header.is_empty() {
        row(f, &table.header, &table.columns, widths, '|', None)?;
    }
    write!(f, "|")?;
    for width in widths {
        write!(f, "{}|", "-".repeat(width + 2))?;
    }
    writeln!(f)?;
    for (index, cells) in table.rows.iter().enumerate() {
        let style = table.row_styles.get(index).cloned();
        row(f, cells, &table.columns, widths, '|', style.as_ref())?;
    }
    Ok(())
}

fn border(
    f: &mut fmt::Formatter<'_>,
    left: char,
    join: char,
    right: char,
    fill: char,
    widths: &[usize],
) -> fmt::Result {
    write!(f, "{left}")?;
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            write!(f, "{join}")?;
        }
        for _ in 0..width + 2 {
            write!(f, "{fill}")?;
        }
    }
    writeln!(f, "{right}")
}

fn row(
    f: &mut fmt::Formatter<'_>,
    cells: &[Cell],
    columns: &[Column],
    widths: &[usize],
    vertical: char,
    row_style: Option<&RowStyle>,
) -> fmt::Result {
    write!(f, "{vertical}")?;
    for (index, width) in widths.iter().enumerate() {
        let mut cell = cells.get(index).cloned().unwrap_or_else(|| Cell::new(""));
        if let Some(style) = row_style {
            if cell.fg.is_none() {
                cell.fg = style.fg;
            }
            if cell.bg.is_none() {
                cell.bg = style.bg;
            }
            for attr in &style.attrs {
                if !cell.attrs.contains(attr) {
                    cell.attrs.push(*attr);
                }
            }
        }
        let rendered = cell.render();
        let visible = display_width(&rendered);
        let text = if visible > *width {
            truncate(&rendered, *width)
        } else {
            rendered
        };
        let visible = display_width(&text);
        let remaining = width.saturating_sub(visible);
        let alignment = columns
            .get(index)
            .map_or(CellAlignment::Left, |column| column.alignment);
        let (before, after) = match alignment {
            CellAlignment::Left => (0, remaining),
            CellAlignment::Center => (remaining / 2, remaining - remaining / 2),
            CellAlignment::Right => (remaining, 0),
        };
        write!(
            f,
            " {}{}{} {vertical}",
            " ".repeat(before),
            text,
            " ".repeat(after)
        )?;
    }
    writeln!(f)
}

fn truncate(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let w = display_width(&ch.to_string());
        if width + w > max_width.saturating_sub(1) {
            result.push('…');
            break;
        }
        result.push(ch);
        width += w;
    }
    result
}

fn terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(80)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_rounded_ansi_aware_table() {
        let mut table = Table::new();
        table.set_header(vec![
            Cell::new("Name").add_attribute(Attribute::Bold),
            Cell::new("State"),
        ]);
        table.add_row(vec![Cell::new("amber"), Cell::new("\x1b[32mok\x1b[0m")]);
        let output = table.to_string();
        assert!(output.starts_with('╭'));
        assert!(output.contains("amber"));
        assert!(output.ends_with("╯\n"));
    }

    #[test]
    fn renders_square_table_with_no_row_rules() {
        let mut table = Table::new();
        table.set_style(TableStyle::Square);
        table.set_header(vec![
            Cell::new("Name")
                .fg(Color::Cyan)
                .add_attribute(Attribute::Bold),
            Cell::new("State"),
        ]);
        table.add_row(vec![Cell::new("ami"), Cell::new("ok")]);
        table.add_row(vec![Cell::new("amber"), Cell::new("ok")]);
        let output = table.to_string();
        assert!(output.starts_with('┌'));
        assert!(output.contains("\x1b[36m\x1b[1mName\x1b[0m"));
        assert_eq!(output.matches('┼').count(), 1);
        assert!(output.ends_with("┘\n"));
    }

    #[test]
    fn renders_ascii_table_matching_tabled_default() {
        let mut table = Table::new();
        table.set_style(TableStyle::Ascii);
        table.set_header(vec![Cell::new("Tool"), Cell::new("Installed")]);
        table.add_row(vec![Cell::new("baby"), Cell::new("1.0")]);
        table.add_row(vec![Cell::new("amber"), Cell::new("2.0")]);
        assert_eq!(
            table.to_string(),
            "+-------+-----------+\n\
             | Tool  | Installed |\n\
             +-------+-----------+\n\
             | baby  | 1.0       |\n\
             +-------+-----------+\n\
             | amber | 2.0       |\n\
             +-------+-----------+\n"
        );
    }

    #[test]
    fn markdown_style_renders_pipe_table() {
        let mut table = Table::new();
        table.set_style(TableStyle::Markdown);
        table.set_header(vec![Cell::new("A"), Cell::new("B")]);
        table.add_row(vec![Cell::new("1"), Cell::new("2")]);
        let output = table.to_string();
        assert!(output.starts_with("| A | B |\n|"));
    }

    #[test]
    fn max_width_constraint_truncates_content() {
        let mut table = Table::new();
        table.set_header(vec![Cell::new("Name"), Cell::new("Description")]);
        table.add_row(vec![Cell::new("x"), Cell::new("a very long description")]);
        table.set_constraints(vec![
            ColumnConstraint::MaxWidth(4),
            ColumnConstraint::MaxWidth(10),
        ]);
        let output = table.to_string();
        assert!(output.contains("a very lo…"));
    }

    #[test]
    fn row_style_applies_to_cells() {
        let mut table = Table::new();
        table.set_header(vec![Cell::new("Name")]);
        table.add_row(vec![Cell::new("item")]);
        table.set_row_style(0, RowStyle::new().fg(Color::Red));
        let output = table.to_string();
        assert!(output.contains("\x1b[31mitem\x1b[0m"));
    }
}
