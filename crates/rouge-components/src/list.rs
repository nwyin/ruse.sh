use rouge_runtime::{Cmd, KeyCode, Msg};
use rouge_style::Style;

use crate::key::Binding;
use crate::textinput::TextInput;

/// Key bindings for the List component.
pub struct ListKeyMap {
    pub cursor_up: Binding,
    pub cursor_down: Binding,
    pub page_up: Binding,
    pub page_down: Binding,
    pub goto_top: Binding,
    pub goto_bottom: Binding,
    pub filter: Binding,
}

impl Default for ListKeyMap {
    fn default() -> Self {
        Self {
            cursor_up: Binding::new(&["up", "k"], "↑/k", "up"),
            cursor_down: Binding::new(&["down", "j"], "↓/j", "down"),
            page_up: Binding::new(&["pgup"], "pgup", "page up"),
            page_down: Binding::new(&["pgdn"], "pgdn", "page down"),
            goto_top: Binding::new(&["home", "g"], "g/home", "go to start"),
            goto_bottom: Binding::new(&["end", "G"], "G/end", "go to end"),
            filter: Binding::new(&["/"], "/", "filter"),
        }
    }
}

/// Trait for items that can be displayed in a list.
pub trait ListItem: Send {
    fn filter_value(&self) -> &str;
    fn title(&self) -> &str;
    fn description(&self) -> &str {
        ""
    }
}

/// A simple list item with title, description, and filter value.
pub struct SimpleItem {
    pub title: String,
    pub desc: String,
    pub filter_val: String,
}

impl SimpleItem {
    pub fn new(title: &str, desc: &str) -> Self {
        Self {
            title: title.to_string(),
            desc: desc.to_string(),
            filter_val: title.to_string(),
        }
    }
}

impl ListItem for SimpleItem {
    fn filter_value(&self) -> &str {
        &self.filter_val
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn description(&self) -> &str {
        &self.desc
    }
}

pub struct ListStyles {
    pub title: Style,
    pub item: Style,
    pub selected_item: Style,
    pub filter_prompt: Style,
    pub status_bar: Style,
}

impl Default for ListStyles {
    fn default() -> Self {
        Self {
            title: Style::new().bold(true),
            item: Style::new(),
            selected_item: Style::new().reverse(true),
            filter_prompt: Style::new().faint(true),
            status_bar: Style::new().faint(true),
        }
    }
}

pub struct List {
    pub key_map: ListKeyMap,
    items: Vec<Box<dyn ListItem>>,
    filtered_indices: Vec<usize>,
    cursor: usize,
    filter_input: TextInput,
    filtering: bool,
    filter_text: String,
    width: usize,
    height: usize,
    y_offset: usize,
    title: String,
    styles: ListStyles,
    pub show_title: bool,
    pub show_filter: bool,
    pub show_status_bar: bool,
}

impl List {
    pub fn new(items: Vec<Box<dyn ListItem>>, width: usize, height: usize) -> Self {
        let count = items.len();
        let indices: Vec<usize> = (0..count).collect();
        Self {
            key_map: ListKeyMap::default(),
            items,
            filtered_indices: indices,
            cursor: 0,
            filter_input: TextInput::new()
                .with_prompt("Filter: ")
                .with_width(width),
            filtering: false,
            filter_text: String::new(),
            width,
            height,
            y_offset: 0,
            title: String::new(),
            styles: ListStyles::default(),
            show_title: true,
            show_filter: true,
            show_status_bar: true,
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn with_styles(mut self, styles: ListStyles) -> Self {
        self.styles = styles;
        self
    }

    pub fn selected_item(&self) -> Option<&dyn ListItem> {
        let idx = self.filtered_indices.get(self.cursor)?;
        Some(self.items[*idx].as_ref())
    }

    pub fn selected_index(&self) -> usize {
        self.cursor
    }

    pub fn set_items(&mut self, items: Vec<Box<dyn ListItem>>) {
        self.items = items;
        self.apply_filter();
        if self.cursor >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
            self.cursor = self.filtered_indices.len() - 1;
        }
    }

    pub fn set_filter_text(&mut self, text: &str) {
        self.filter_text = text.to_string();
        self.filter_input.set_value(text);
        self.apply_filter();
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        if self.filtering {
            return self.update_filtering(msg);
        }

        if let Msg::KeyPress(key) = msg {
            if self.key_map.cursor_up.matches(key) {
                self.cursor_up();
            } else if self.key_map.cursor_down.matches(key) {
                self.cursor_down();
            } else if self.key_map.page_up.matches(key) {
                for _ in 0..self.visible_item_count() {
                    self.cursor_up();
                }
            } else if self.key_map.page_down.matches(key) {
                for _ in 0..self.visible_item_count() {
                    self.cursor_down();
                }
            } else if self.key_map.goto_top.matches(key) {
                self.cursor = 0;
                self.y_offset = 0;
            } else if self.key_map.goto_bottom.matches(key) {
                if !self.filtered_indices.is_empty() {
                    self.cursor = self.filtered_indices.len() - 1;
                    self.ensure_cursor_visible();
                }
            } else if self.show_filter && self.key_map.filter.matches(key) {
                self.filtering = true;
                self.filter_input.set_value(&self.filter_text);
                return self.filter_input.focus();
            }
        }
        None
    }

    pub fn view(&self) -> String {
        let mut out = String::new();
        let mut used_height = 0;

        // Title
        if self.show_title && !self.title.is_empty() {
            out.push_str(&self.styles.title.render(&[&self.title]));
            out.push('\n');
            used_height += 1;
        }

        // Filter input (when filtering)
        if self.filtering {
            out.push_str(&self.filter_input.view());
            out.push('\n');
            used_height += 1;
        } else if !self.filter_text.is_empty() && self.show_filter {
            let filter_display = format!("Filter: {}", self.filter_text);
            out.push_str(&self.styles.filter_prompt.render(&[&filter_display]));
            out.push('\n');
            used_height += 1;
        }

        // Status bar
        if self.show_status_bar {
            used_height += 1; // reserve space for status bar at the bottom
        }

        let item_height = self.height.saturating_sub(used_height);
        let end = (self.y_offset + item_height).min(self.filtered_indices.len());

        // Items
        for i in self.y_offset..end {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            let item_idx = self.filtered_indices[i];
            let item = &self.items[item_idx];

            let style = if i == self.cursor {
                &self.styles.selected_item
            } else {
                &self.styles.item
            };

            let mut line = item.title().to_string();
            let desc = item.description();
            if !desc.is_empty() {
                line.push_str(&format!(" - {desc}"));
            }

            // Truncate to width
            if self.width > 0 && rouge_ansi::string_width(&line) > self.width {
                line = rouge_ansi::truncate(&line, self.width, "...");
            }

            out.push_str(&style.render(&[&line]));
        }

        // Pad remaining item space
        for _ in (end.saturating_sub(self.y_offset))..item_height {
            out.push('\n');
        }

        // Status bar
        if self.show_status_bar {
            out.push('\n');
            let total = self.items.len();
            let filtered = self.filtered_indices.len();
            let status = if filtered == total {
                format!("{total} items")
            } else {
                format!("{filtered}/{total} items")
            };
            out.push_str(&self.styles.status_bar.render(&[&status]));
        }

        out
    }

    fn update_filtering(&mut self, msg: &Msg) -> Cmd {
        if let Msg::KeyPress(key) = msg {
            match key.code {
                KeyCode::Enter => {
                    // Accept filter
                    self.filter_text = self.filter_input.value();
                    self.filtering = false;
                    self.filter_input.blur();
                    self.apply_filter();
                    return None;
                }
                KeyCode::Escape => {
                    // Cancel filter
                    self.filtering = false;
                    self.filter_input.blur();
                    return None;
                }
                _ => {}
            }
        }

        let cmd = self.filter_input.update(msg);
        // Live filter as user types
        self.filter_text = self.filter_input.value();
        self.apply_filter();
        cmd
    }

    fn apply_filter(&mut self) {
        if self.filter_text.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            let filter = self.filter_text.to_lowercase();
            self.filtered_indices = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.filter_value().to_lowercase().contains(&filter))
                .map(|(i, _)| i)
                .collect();
        }

        if self.cursor >= self.filtered_indices.len() {
            self.cursor = self.filtered_indices.len().saturating_sub(1);
        }
        self.ensure_cursor_visible();
    }

    fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.ensure_cursor_visible();
        }
    }

    fn cursor_down(&mut self) {
        if self.cursor < self.filtered_indices.len().saturating_sub(1) {
            self.cursor += 1;
            self.ensure_cursor_visible();
        }
    }

    fn visible_item_count(&self) -> usize {
        let mut used = 0;
        if self.show_title && !self.title.is_empty() {
            used += 1;
        }
        if self.filtering || (!self.filter_text.is_empty() && self.show_filter) {
            used += 1;
        }
        if self.show_status_bar {
            used += 1;
        }
        self.height.saturating_sub(used)
    }

    fn ensure_cursor_visible(&mut self) {
        let visible = self.visible_item_count();
        if visible == 0 {
            return;
        }
        if self.cursor < self.y_offset {
            self.y_offset = self.cursor;
        } else if self.cursor >= self.y_offset + visible {
            self.y_offset = self.cursor - visible + 1;
        }
    }
}
