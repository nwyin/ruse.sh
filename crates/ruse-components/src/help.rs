use ruse_style::Style;

use crate::key::{Binding, KeyMap};

pub struct Help {
    pub show_all: bool,
    pub short_separator: String,
    pub full_separator: String,
    pub width: usize,
    pub key_style: Style,
    pub desc_style: Style,
    pub sep_style: Style,
}

impl Default for Help {
    fn default() -> Self {
        Self::new()
    }
}

impl Help {
    pub fn new() -> Self {
        Self {
            show_all: false,
            short_separator: " \u{2022} ".to_string(), // bullet
            full_separator: "    ".to_string(),
            width: 80,
            key_style: Style::new().bold(true),
            desc_style: Style::new(),
            sep_style: Style::new().faint(true),
        }
    }

    pub fn set_width(&mut self, w: usize) {
        self.width = w;
    }

    pub fn view_key_map(&self, km: &dyn KeyMap) -> String {
        if self.show_all {
            let groups = km.full_help();
            self.full_help_view(&groups)
        } else {
            let bindings = km.short_help();
            self.short_help_view(&bindings)
        }
    }

    pub fn short_help_view(&self, bindings: &[&Binding]) -> String {
        let enabled: Vec<&&Binding> = bindings.iter().filter(|b| b.enabled()).collect();
        if enabled.is_empty() {
            return String::new();
        }

        let sep = self.sep_style.render(&[&self.short_separator]);

        let mut parts: Vec<String> = Vec::new();
        let mut total_width = 0;

        for binding in &enabled {
            let key = self.key_style.render(&[binding.help_key()]);
            let desc = self.desc_style.render(&[binding.help_desc()]);
            let part = format!("{key} {desc}");

            let part_width = ruse_ansi::string_width(&part);
            let sep_width = if parts.is_empty() { 0 } else { ruse_ansi::string_width(&self.short_separator) };

            if !parts.is_empty() && total_width + sep_width + part_width > self.width {
                break;
            }

            total_width += if parts.is_empty() { 0 } else { sep_width } + part_width;
            parts.push(part);
        }

        parts.join(&sep)
    }

    pub fn full_help_view(&self, groups: &[Vec<&Binding>]) -> String {
        // Build each column as a Vec of rendered lines, tracking the
        // maximum visible width for each column so we can pad them.
        let mut col_lines: Vec<Vec<String>> = Vec::new();
        let mut col_widths: Vec<usize> = Vec::new();

        for group in groups {
            let enabled: Vec<&&Binding> = group.iter().filter(|b| b.enabled()).collect();
            if enabled.is_empty() {
                continue;
            }

            let mut lines: Vec<String> = Vec::new();
            let mut max_w: usize = 0;
            for binding in &enabled {
                let key = self.key_style.render(&[binding.help_key()]);
                let desc = self.desc_style.render(&[binding.help_desc()]);
                let line = format!("{key} {desc}");
                let w = ruse_ansi::string_width(&line);
                if w > max_w {
                    max_w = w;
                }
                lines.push(line);
            }
            col_lines.push(lines);
            col_widths.push(max_w);
        }

        if col_lines.is_empty() {
            return String::new();
        }

        let sep = self.sep_style.render(&[&self.full_separator]);
        let sep_width = ruse_ansi::string_width(&self.full_separator);

        // Determine how many rows we need (the tallest column)
        let max_rows = col_lines.iter().map(|c| c.len()).max().unwrap_or(0);

        // Render row by row, placing columns side by side
        let mut out = String::new();
        for row in 0..max_rows {
            if row > 0 {
                out.push('\n');
            }
            for (ci, col) in col_lines.iter().enumerate() {
                if ci > 0 {
                    out.push_str(&sep);
                }
                if row < col.len() {
                    let line = &col[row];
                    out.push_str(line);
                    // Pad to column width so the next column aligns
                    if ci < col_lines.len() - 1 {
                        let w = ruse_ansi::string_width(line);
                        let pad = col_widths[ci].saturating_sub(w);
                        for _ in 0..pad {
                            out.push(' ');
                        }
                    }
                } else if ci < col_lines.len() - 1 {
                    // Empty cell; pad full column width
                    for _ in 0..col_widths[ci] {
                        out.push(' ');
                    }
                }
            }
            // Check if the row exceeds the configured width; if so, stop
            // (we still include the current row for completeness).
            let row_width: usize = col_widths.iter().sum::<usize>()
                + (col_lines.len().saturating_sub(1)) * sep_width;
            let _ = row_width; // available for future truncation
        }

        out
    }
}
