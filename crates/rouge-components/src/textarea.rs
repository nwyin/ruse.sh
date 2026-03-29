use rouge_runtime::{Cmd, KeyCode, Modifiers, Msg};
use rouge_style::Style;

pub struct TextArea {
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
            let ctrl = key.modifiers.contains(Modifiers::CTRL);
            match key.code {
                KeyCode::Char(ch) if !ctrl => {
                    self.insert_char(ch);
                }
                KeyCode::Enter => {
                    self.insert_newline();
                }
                KeyCode::Backspace => {
                    self.delete_before_cursor();
                }
                KeyCode::Delete => {
                    self.delete_after_cursor();
                }
                KeyCode::Left => {
                    self.cursor_left();
                }
                KeyCode::Right => {
                    self.cursor_right();
                }
                KeyCode::Up => {
                    self.cursor_up();
                }
                KeyCode::Down => {
                    self.cursor_down();
                }
                KeyCode::Home => {
                    self.cursor_col = 0;
                }
                KeyCode::End => {
                    self.cursor_col = self.current_line_len();
                }
                KeyCode::PageUp => {
                    for _ in 0..self.height {
                        self.cursor_up();
                    }
                }
                KeyCode::PageDown => {
                    for _ in 0..self.height {
                        self.cursor_down();
                    }
                }
                KeyCode::Tab => {
                    // Insert 4 spaces
                    for _ in 0..4 {
                        self.insert_char(' ');
                    }
                }
                _ => {}
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
            let display = if content_width > 0 && rouge_ansi::string_width(&line) > content_width {
                rouge_ansi::truncate(&line, content_width, "")
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
                let visible_width = rouge_ansi::string_width(&line_out);
                if content_width > visible_width {
                    line_out.push_str(&" ".repeat(content_width - visible_width));
                }
                out.push_str(&line_out);
            } else {
                out.push_str(&display);
                // Pad to content width
                let visible_width = rouge_ansi::string_width(&display);
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
