use ruse_ansi::string_width;

use crate::border::{Border, ROUNDED_BORDER};
use crate::render::get_lines;
use crate::style::Style;

/// Row index constant representing the header row in `style_func`.
pub const HEADER_ROW: i32 = -1;

/// A static table renderer.
///
/// Build a table with headers, rows, optional borders, and per-cell styling,
/// then call [`Table::render`] to produce a string.
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    border: Border,
    border_top: bool,
    border_bottom: bool,
    border_left: bool,
    border_right: bool,
    border_header: bool,
    border_column: bool,
    border_row: bool,
    border_style: Style,
    width: usize,
    style_func: Option<Box<dyn Fn(i32, usize) -> Style>>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            border: ROUNDED_BORDER,
            border_top: true,
            border_bottom: true,
            border_left: true,
            border_right: true,
            border_header: true,
            border_column: true,
            border_row: false,
            border_style: Style::new(),
            width: 0,
            style_func: None,
        }
    }

    pub fn headers(mut self, headers: Vec<String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.rows = rows;
        self
    }

    pub fn row(mut self, row: Vec<String>) -> Self {
        self.rows.push(row);
        self
    }

    pub fn border(mut self, b: Border) -> Self {
        self.border = b;
        self
    }

    pub fn border_top(mut self, v: bool) -> Self {
        self.border_top = v;
        self
    }

    pub fn border_bottom(mut self, v: bool) -> Self {
        self.border_bottom = v;
        self
    }

    pub fn border_left(mut self, v: bool) -> Self {
        self.border_left = v;
        self
    }

    pub fn border_right(mut self, v: bool) -> Self {
        self.border_right = v;
        self
    }

    pub fn border_header(mut self, v: bool) -> Self {
        self.border_header = v;
        self
    }

    pub fn border_column(mut self, v: bool) -> Self {
        self.border_column = v;
        self
    }

    pub fn border_row(mut self, v: bool) -> Self {
        self.border_row = v;
        self
    }

    pub fn border_style(mut self, s: Style) -> Self {
        self.border_style = s;
        self
    }

    pub fn width(mut self, w: usize) -> Self {
        self.width = w;
        self
    }

    pub fn style_func<F: Fn(i32, usize) -> Style + 'static>(mut self, f: F) -> Self {
        self.style_func = Some(Box::new(f));
        self
    }

    /// Render the table to a string.
    pub fn render(&self) -> String {
        let num_cols = self.num_cols();
        if num_cols == 0 {
            return String::new();
        }

        // Calculate column widths
        let col_widths = self.calc_col_widths(num_cols);

        // Render all cells
        let header_cells: Vec<String> = (0..num_cols)
            .map(|c| self.render_cell(HEADER_ROW, c, &col_widths))
            .collect();

        let row_cells: Vec<Vec<String>> = (0..self.rows.len())
            .map(|r| {
                (0..num_cols)
                    .map(|c| self.render_cell(r as i32, c, &col_widths))
                    .collect()
            })
            .collect();

        let mut out = String::new();

        // Top border
        if self.border_top {
            out.push_str(&self.render_horizontal_rule(
                &col_widths,
                self.border.top_left,
                self.border.top,
                self.border.middle_top,
                self.border.top_right,
            ));
            out.push('\n');
        }

        // Header row
        if !self.headers.is_empty() {
            let header_height = header_cells
                .iter()
                .map(|c| cell_height(c))
                .max()
                .unwrap_or(1);
            for line_idx in 0..header_height {
                out.push_str(&self.render_data_line(&header_cells, &col_widths, line_idx));
                out.push('\n');
            }

            // Header separator
            if self.border_header {
                out.push_str(&self.render_horizontal_rule(
                    &col_widths,
                    self.border.middle_left,
                    self.border.top, // Use top char for header separator
                    self.border.middle,
                    self.border.middle_right,
                ));
                out.push('\n');
            }
        }

        // Data rows
        for (r, cells) in row_cells.iter().enumerate() {
            let row_height = cells.iter().map(|c| cell_height(c)).max().unwrap_or(1);
            for line_idx in 0..row_height {
                out.push_str(&self.render_data_line(cells, &col_widths, line_idx));
                out.push('\n');
            }

            // Row separator (not after last row)
            if self.border_row && r < row_cells.len() - 1 {
                out.push_str(&self.render_horizontal_rule(
                    &col_widths,
                    self.border.middle_left,
                    self.border.bottom, // Use bottom char for row separators
                    self.border.middle,
                    self.border.middle_right,
                ));
                out.push('\n');
            }
        }

        // Bottom border
        if self.border_bottom {
            out.push_str(&self.render_horizontal_rule(
                &col_widths,
                self.border.bottom_left,
                self.border.bottom,
                self.border.middle_bottom,
                self.border.bottom_right,
            ));
            out.push('\n');
        }

        // Remove trailing newline
        if out.ends_with('\n') {
            out.pop();
        }

        out
    }

    fn num_cols(&self) -> usize {
        let mut n = self.headers.len();
        for row in &self.rows {
            n = n.max(row.len());
        }
        n
    }

    fn calc_col_widths(&self, num_cols: usize) -> Vec<usize> {
        // Start with natural content widths
        let mut widths = vec![0usize; num_cols];

        for (c, h) in self.headers.iter().enumerate() {
            widths[c] = widths[c].max(string_width(h));
        }

        for row in &self.rows {
            for (c, cell) in row.iter().enumerate() {
                if c < num_cols {
                    widths[c] = widths[c].max(string_width(cell));
                }
            }
        }

        // If a total width is set, distribute evenly
        if self.width > 0 {
            let border_overhead = self.border_width_overhead(num_cols);
            let available = self.width.saturating_sub(border_overhead);

            if available > 0 {
                let per_col = available / num_cols;
                let mut remainder = available % num_cols;
                for w in widths.iter_mut() {
                    *w = per_col;
                    if remainder > 0 {
                        *w += 1;
                        remainder -= 1;
                    }
                }
            }
        }

        widths
    }

    fn border_width_overhead(&self, num_cols: usize) -> usize {
        let mut overhead = 0;
        if self.border_left {
            overhead += string_width(self.border.left);
        }
        if self.border_right {
            overhead += string_width(self.border.right);
        }
        if self.border_column && num_cols > 1 {
            overhead += string_width(self.border.left) * (num_cols - 1);
        }
        overhead
    }

    fn get_cell_content(&self, row: i32, col: usize) -> &str {
        if row == HEADER_ROW {
            self.headers.get(col).map_or("", |s| s.as_str())
        } else {
            self.rows
                .get(row as usize)
                .and_then(|r| r.get(col))
                .map_or("", |s| s.as_str())
        }
    }

    fn render_cell(&self, row: i32, col: usize, col_widths: &[usize]) -> String {
        let content = self.get_cell_content(row, col);
        let target_width = col_widths[col];

        let style = self
            .style_func
            .as_ref()
            .map_or_else(Style::new, |f| f(row, col));

        // Apply style to content, then pad to target width
        let styled = style.render_one(content);

        // Pad each line to target width
        let (lines, _) = get_lines(&styled);
        let mut out = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(line);
            let line_w = string_width(line);
            if line_w < target_width {
                out.push_str(&" ".repeat(target_width - line_w));
            }
        }
        out
    }

    fn render_data_line(&self, cells: &[String], col_widths: &[usize], line_idx: usize) -> String {
        let mut out = String::new();

        if self.border_left {
            out.push_str(self.border.left);
        }

        for (c, cell) in cells.iter().enumerate() {
            if c > 0 && self.border_column {
                out.push_str(self.border.left);
            }

            let cell_lines: Vec<&str> = cell.split('\n').collect();
            let line = cell_lines.get(line_idx).copied().unwrap_or("");
            out.push_str(line);

            // Pad if this line is shorter than the column width
            let line_w = string_width(line);
            let target = col_widths[c];
            if line_w < target {
                out.push_str(&" ".repeat(target - line_w));
            }
        }

        if self.border_right {
            out.push_str(self.border.right);
        }

        out
    }

    fn render_horizontal_rule(
        &self,
        col_widths: &[usize],
        left: &str,
        middle: &str,
        cross: &str,
        right: &str,
    ) -> String {
        let mut out = String::new();

        if self.border_left {
            out.push_str(left);
        }

        let mid = if middle.is_empty() { " " } else { middle };

        for (c, &w) in col_widths.iter().enumerate() {
            if c > 0 && self.border_column {
                out.push_str(cross);
            }
            out.push_str(&mid.repeat(w));
        }

        if self.border_right {
            out.push_str(right);
        }

        out
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

fn cell_height(s: &str) -> usize {
    if s.is_empty() {
        return 1;
    }
    s.chars().filter(|&c| c == '\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::border::{ASCII_BORDER, NO_BORDER};

    #[test]
    fn test_basic_table() {
        let t = Table::new()
            .border(ASCII_BORDER)
            .headers(vec!["Name".into(), "Age".into()])
            .row(vec!["Alice".into(), "30".into()])
            .row(vec!["Bob".into(), "25".into()]);
        let result = t.render();
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
        assert!(result.contains("+"));
        assert!(result.contains("|"));
    }

    #[test]
    fn test_no_border_table() {
        let t = Table::new()
            .border(NO_BORDER)
            .border_top(false)
            .border_bottom(false)
            .border_left(false)
            .border_right(false)
            .border_column(false)
            .border_header(false)
            .headers(vec!["A".into(), "B".into()])
            .row(vec!["1".into(), "2".into()]);
        let result = t.render();
        assert!(result.contains("A"));
        assert!(result.contains("1"));
    }

    #[test]
    fn test_empty_table() {
        let t = Table::new();
        assert_eq!(t.render(), "");
    }

    #[test]
    fn test_fixed_width_table() {
        let t = Table::new()
            .border(ASCII_BORDER)
            .width(30)
            .headers(vec!["Name".into(), "Value".into()])
            .row(vec!["a".into(), "b".into()]);
        let result = t.render();
        let lines: Vec<&str> = result.split('\n').collect();
        // All lines should be the same width
        let widths: Vec<usize> = lines.iter().map(|l| string_width(l)).collect();
        assert!(widths.iter().all(|&w| w == widths[0]));
    }

    #[test]
    fn test_style_func() {
        let t = Table::new()
            .border(ASCII_BORDER)
            .headers(vec!["H".into()])
            .row(vec!["data".into()])
            .style_func(|row, _col| {
                if row == HEADER_ROW {
                    Style::new().bold(true)
                } else {
                    Style::new()
                }
            });
        let result = t.render();
        // Header should be bold (contains ANSI bold sequence)
        assert!(result.contains("\x1b[1m"));
    }
}
