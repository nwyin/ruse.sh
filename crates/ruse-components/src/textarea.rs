use ruse_runtime::{Cmd, KeyCode, Modifiers, Msg};
use ruse_style::Style;

use crate::key::Binding;

/// Key bindings for the TextArea component.
pub struct TextAreaKeyMap {
    pub char_forward: Binding,
    pub char_backward: Binding,
    pub word_forward: Binding,
    pub word_backward: Binding,
    pub char_up: Binding,
    pub char_down: Binding,
    pub delete_char_backward: Binding,
    pub delete_char_forward: Binding,
    pub delete_word_backward: Binding,
    pub line_start: Binding,
    pub line_end: Binding,
    pub delete_line: Binding,
    pub insert_newline: Binding,
    pub delete_before_cursor: Binding,
    pub page_up: Binding,
    pub page_down: Binding,
    pub goto_top: Binding,
    pub goto_bottom: Binding,
}

impl Default for TextAreaKeyMap {
    fn default() -> Self {
        Self {
            char_forward: Binding::new(&["right", "ctrl+f"], "→", "forward"),
            char_backward: Binding::new(&["left", "ctrl+b"], "←", "backward"),
            word_forward: Binding::new(&["alt+right", "ctrl+right", "alt+f"], "alt+→", "word forward"),
            word_backward: Binding::new(&["alt+left", "ctrl+left", "alt+b"], "alt+←", "word backward"),
            char_up: Binding::new(&["up"], "↑", "up"),
            char_down: Binding::new(&["down"], "↓", "down"),
            delete_char_backward: Binding::new(&["backspace"], "bksp", "delete char"),
            delete_char_forward: Binding::new(&["delete"], "del", "delete char forward"),
            delete_word_backward: Binding::new(&["ctrl+w"], "ctrl+w", "delete word"),
            line_start: Binding::new(&["home", "ctrl+a"], "home", "line start"),
            line_end: Binding::new(&["end", "ctrl+e"], "end", "line end"),
            delete_line: Binding::new(&["ctrl+k"], "ctrl+k", "delete line"),
            insert_newline: Binding::new(&["enter"], "enter", "new line"),
            delete_before_cursor: Binding::new(&["ctrl+u"], "ctrl+u", "delete to start"),
            page_up: Binding::new(&["pgup"], "pgup", "page up"),
            page_down: Binding::new(&["pgdn"], "pgdn", "page down"),
            goto_top: Binding::new(&["ctrl+home"], "ctrl+home", "go to start"),
            goto_bottom: Binding::new(&["ctrl+end"], "ctrl+end", "go to end"),
        }
    }
}

pub struct TextArea {
    pub key_map: TextAreaKeyMap,
    lines: Vec<Vec<char>>,
    cursor_row: usize,
    cursor_col: usize,
    width: usize,
    height: usize,
    y_offset: usize,
    focus: bool,
    pub show_line_numbers: bool,
    #[allow(dead_code)]
    style: Style,
    pub line_number_style: Style,
    pub cursor_line_style: Style,
}

impl Default for TextArea {
    fn default() -> Self {
        Self::new()
    }
}

impl TextArea {
    pub fn new() -> Self {
        Self {
            key_map: TextAreaKeyMap::default(),
            lines: vec![Vec::new()],
            cursor_row: 0,
            cursor_col: 0,
            width: 80,
            height: 10,
            y_offset: 0,
            focus: false,
            show_line_numbers: true,
            style: Style::new(),
            line_number_style: Style::new().faint(true),
            cursor_line_style: Style::new(),
        }
    }

    pub fn set_width(&mut self, w: usize) {
        self.width = w;
    }

    pub fn set_height(&mut self, h: usize) {
        self.height = h;
        self.ensure_cursor_visible();
    }

    pub fn value(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn set_value(&mut self, s: &str) {
        self.lines = s.lines().map(|l| l.chars().collect()).collect();
        if self.lines.is_empty() {
            self.lines.push(Vec::new());
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.y_offset = 0;
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    pub fn focus(&mut self) -> Cmd {
        self.focus = true;
        None
    }

    pub fn blur(&mut self) {
        self.focus = false;
    }

    pub fn focused(&self) -> bool {
        self.focus
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        if !self.focus {
            return None;
        }

        if let Msg::KeyPress(key) = msg {
            if self.key_map.insert_newline.matches(key) {
                self.insert_newline();
            } else if self.key_map.delete_char_backward.matches(key) {
                self.delete_before_cursor();
            } else if self.key_map.delete_char_forward.matches(key) {
                self.delete_after_cursor();
            } else if self.key_map.delete_word_backward.matches(key) {
                self.delete_word_before_cursor();
            } else if self.key_map.delete_before_cursor.matches(key) {
                self.delete_to_line_start();
            } else if self.key_map.delete_line.matches(key) {
                self.delete_to_line_end();
            } else if self.key_map.word_forward.matches(key) {
                self.cursor_word_right();
            } else if self.key_map.word_backward.matches(key) {
                self.cursor_word_left();
            } else if self.key_map.char_backward.matches(key) {
                self.cursor_left();
            } else if self.key_map.char_forward.matches(key) {
                self.cursor_right();
            } else if self.key_map.char_up.matches(key) {
                self.cursor_up();
            } else if self.key_map.char_down.matches(key) {
                self.cursor_down();
            } else if self.key_map.line_start.matches(key) {
                self.cursor_col = 0;
            } else if self.key_map.line_end.matches(key) {
                self.cursor_col = self.current_line_len();
            } else if self.key_map.page_up.matches(key) {
                for _ in 0..self.height {
                    self.cursor_up();
                }
            } else if self.key_map.page_down.matches(key) {
                for _ in 0..self.height {
                    self.cursor_down();
                }
            } else if self.key_map.goto_top.matches(key) {
                self.cursor_row = 0;
                self.cursor_col = 0;
            } else if self.key_map.goto_bottom.matches(key) {
                self.cursor_row = self.lines.len().saturating_sub(1);
                self.cursor_col = self.current_line_len();
            } else if key.code == KeyCode::Tab {
                // Insert 4 spaces
                for _ in 0..4 {
                    self.insert_char(' ');
                }
            } else if let KeyCode::Char(ch) = key.code {
                // Insert normal characters (no ctrl/alt modifiers)
                if !key.modifiers.contains(Modifiers::CTRL) && !key.modifiers.contains(Modifiers::ALT) {
                    self.insert_char(ch);
                }
            }
            self.ensure_cursor_visible();
        }
        None
    }

    pub fn view(&self) -> String {
        let gutter_width = if self.show_line_numbers {
            let digits = format!("{}", self.lines.len()).len();
            digits + 2 // number + space + separator
        } else {
            0
        };

        let content_width = self.width.saturating_sub(gutter_width);
        let end = (self.y_offset + self.height).min(self.lines.len());

        let mut out = String::new();
        for row in self.y_offset..end {
            if row > self.y_offset {
                out.push('\n');
            }

            // Line number gutter
            if self.show_line_numbers {
                let digits = format!("{}", self.lines.len()).len();
                let num_str = format!("{:>width$} ", row + 1, width = digits);
                out.push_str(&self.line_number_style.render(&[&num_str]));
            }

            // Line content
            let line: String = self.lines[row].iter().collect();
            let display = if content_width > 0 && ruse_ansi::string_width(&line) > content_width {
                ruse_ansi::truncate(&line, content_width, "")
            } else {
                line
            };

            if row == self.cursor_row && self.focus {
                // Render cursor on current line
                let chars: Vec<char> = self.lines[row].to_vec();
                let mut line_out = String::new();
                for (col, ch) in chars.iter().enumerate() {
                    if col == self.cursor_col {
                        let cursor_style = Style::new().reverse(true);
                        line_out.push_str(&cursor_style.render(&[&ch.to_string()]));
                    } else {
                        line_out.push(*ch);
                    }
                }
                if self.cursor_col >= chars.len() {
                    // Cursor past end of line
                    let cursor_style = Style::new().reverse(true);
                    line_out.push_str(&cursor_style.render(&[" "]));
                }
                // Pad to content width
                let visible_width = ruse_ansi::string_width(&line_out);
                if content_width > visible_width {
                    line_out.push_str(&" ".repeat(content_width - visible_width));
                }
                out.push_str(&line_out);
            } else {
                out.push_str(&display);
                // Pad to content width
                let visible_width = ruse_ansi::string_width(&display);
                if content_width > visible_width {
                    out.push_str(&" ".repeat(content_width - visible_width));
                }
            }
        }

        // Pad remaining height
        for _ in (end - self.y_offset)..self.height {
            out.push('\n');
            if self.show_line_numbers {
                let digits = format!("{}", self.lines.len()).len();
                let pad = " ".repeat(digits + 2);
                out.push_str(&self.line_number_style.render(&[&pad]));
            }
            if content_width > 0 {
                out.push_str(&" ".repeat(content_width));
            }
        }

        out
    }

    fn insert_char(&mut self, ch: char) {
        self.lines[self.cursor_row].insert(self.cursor_col, ch);
        self.cursor_col += 1;
    }

    fn insert_newline(&mut self) {
        let rest: Vec<char> = self.lines[self.cursor_row].drain(self.cursor_col..).collect();
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
    }

    fn delete_before_cursor(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            self.lines[self.cursor_row].remove(self.cursor_col);
        } else if self.cursor_row > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].extend(current_line);
        }
    }

    fn delete_after_cursor(&mut self) {
        if self.cursor_col < self.lines[self.cursor_row].len() {
            self.lines[self.cursor_row].remove(self.cursor_col);
        } else if self.cursor_row < self.lines.len() - 1 {
            // Merge next line into current
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].extend(next_line);
        }
    }

    fn cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.current_line_len();
        }
    }

    fn cursor_right(&mut self) {
        if self.cursor_col < self.current_line_len() {
            self.cursor_col += 1;
        } else if self.cursor_row < self.lines.len() - 1 {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn cursor_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.cursor_col.min(self.current_line_len());
        }
    }

    fn cursor_down(&mut self) {
        if self.cursor_row < self.lines.len() - 1 {
            self.cursor_row += 1;
            self.cursor_col = self.cursor_col.min(self.current_line_len());
        }
    }

    fn cursor_word_right(&mut self) {
        let line = &self.lines[self.cursor_row];
        let len = line.len();
        if self.cursor_col >= len {
            // Move to next line if possible
            if self.cursor_row < self.lines.len() - 1 {
                self.cursor_row += 1;
                self.cursor_col = 0;
            }
            return;
        }
        // Skip current word chars
        while self.cursor_col < len && line[self.cursor_col] != ' ' {
            self.cursor_col += 1;
        }
        // Skip whitespace
        while self.cursor_col < len && line[self.cursor_col] == ' ' {
            self.cursor_col += 1;
        }
    }

    fn cursor_word_left(&mut self) {
        if self.cursor_col == 0 {
            // Move to end of previous line if possible
            if self.cursor_row > 0 {
                self.cursor_row -= 1;
                self.cursor_col = self.current_line_len();
            }
            return;
        }
        let line = &self.lines[self.cursor_row];
        // Skip whitespace
        while self.cursor_col > 0 && line[self.cursor_col - 1] == ' ' {
            self.cursor_col -= 1;
        }
        // Skip word chars
        while self.cursor_col > 0 && line[self.cursor_col - 1] != ' ' {
            self.cursor_col -= 1;
        }
    }

    fn delete_word_before_cursor(&mut self) {
        if self.cursor_col == 0 {
            return;
        }
        let line = &self.lines[self.cursor_row];
        let mut end = self.cursor_col;
        // Skip whitespace
        while end > 0 && line[end - 1] == ' ' {
            end -= 1;
        }
        // Skip word chars
        while end > 0 && line[end - 1] != ' ' {
            end -= 1;
        }
        self.lines[self.cursor_row].drain(end..self.cursor_col);
        self.cursor_col = end;
    }

    fn delete_to_line_start(&mut self) {
        self.lines[self.cursor_row].drain(0..self.cursor_col);
        self.cursor_col = 0;
    }

    fn delete_to_line_end(&mut self) {
        self.lines[self.cursor_row].truncate(self.cursor_col);
    }

    fn current_line_len(&self) -> usize {
        self.lines[self.cursor_row].len()
    }

    fn ensure_cursor_visible(&mut self) {
        if self.cursor_row < self.y_offset {
            self.y_offset = self.cursor_row;
        } else if self.cursor_row >= self.y_offset + self.height {
            self.y_offset = self.cursor_row - self.height + 1;
        }
    }
}
