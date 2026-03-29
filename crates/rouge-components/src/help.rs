use rouge_style::Style;

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
            let key = self.key_style.render(&[&binding.help_key]);
            let desc = self.desc_style.render(&[&binding.help_desc]);
            let part = format!("{key} {desc}");

            let part_width = rouge_ansi::string_width(&part);
            let sep_width = if parts.is_empty() { 0 } else { rouge_ansi::string_width(&self.short_separator) };

            if !parts.is_empty() && total_width + sep_width + part_width > self.width {
                break;
            }

            total_width += if parts.is_empty() { 0 } else { sep_width } + part_width;
            parts.push(part);
        }

        parts.join(&sep)
    }

    pub fn full_help_view(&self, groups: &[Vec<&Binding>]) -> String {
        let mut columns: Vec<String> = Vec::new();

        for group in groups {
            let enabled: Vec<&&Binding> = group.iter().filter(|b| b.enabled()).collect();
            if enabled.is_empty() {
                continue;
            }

            let mut lines: Vec<String> = Vec::new();
            for binding in &enabled {
                let key = self.key_style.render(&[&binding.help_key]);
                let desc = self.desc_style.render(&[&binding.help_desc]);
                lines.push(format!("{key} {desc}"));
            }
            columns.push(lines.join("\n"));
        }

        let sep = self.sep_style.render(&[&self.full_separator]);
        // Join columns side by side, separated by the full separator
        // For simplicity, lay them out vertically with blank lines between groups
        columns.join(&format!("\n{sep}\n"))
    }
}
