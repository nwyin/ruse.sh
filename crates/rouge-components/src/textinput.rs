use rouge_runtime::{Cmd, KeyCode, Modifiers, Msg};
use rouge_style::Style;

use crate::cursor::{Cursor, CursorMode};
use crate::key::Binding;

#[derive(Clone, Copy, PartialEq)]
pub enum EchoMode {
    Normal,
    Password,
    None,
}

/// Key bindings for the TextInput component.
pub struct TextInputKeyMap {
    pub char_forward: Binding,
    pub char_backward: Binding,
    pub word_forward: Binding,
    pub word_backward: Binding,
    pub delete_char_backward: Binding,
    pub delete_char_forward: Binding,
    pub delete_word_backward: Binding,
    pub line_start: Binding,
    pub line_end: Binding,
    pub kill_line: Binding,
    pub delete_before_cursor: Binding,
}

impl Default for TextInputKeyMap {
    fn default() -> Self {
        Self {
            char_forward: Binding::new(&["right", "ctrl+f"], "→", "forward"),
            char_backward: Binding::new(&["left", "ctrl+b"], "←", "backward"),
            word_forward: Binding::new(&["alt+right", "ctrl+right", "alt+f"], "alt+→", "word forward"),
            word_backward: Binding::new(&["alt+left", "ctrl+left", "alt+b"], "alt+←", "word backward"),
            delete_char_backward: Binding::new(&["backspace"], "bksp", "delete char"),
            delete_char_forward: Binding::new(&["delete"], "del", "delete char forward"),
            delete_word_backward: Binding::new(&["ctrl+w"], "ctrl+w", "delete word"),
            line_start: Binding::new(&["home", "ctrl+a"], "home", "line start"),
            line_end: Binding::new(&["end", "ctrl+e"], "end", "line end"),
            kill_line: Binding::new(&["ctrl+k"], "ctrl+k", "kill line"),
            delete_before_cursor: Binding::new(&["ctrl+u"], "ctrl+u", "delete to start"),
        }
    }
}

pub struct TextInput {
    pub key_map: TextInputKeyMap,
    value: Vec<char>,
    pos: usize,
    offset: usize,
    offset_right: usize,
    width: usize,
    prompt: String,
    placeholder: String,
    echo_mode: EchoMode,
    echo_char: char,
    focus: bool,
    cursor: Cursor,
    char_limit: usize,
    style: Style,
    placeholder_style: Style,
    #[allow(dead_code)]
    cursor_style: Style,
    suggestions: Vec<String>,
    matched_suggestions: Vec<String>,
    current_suggestion_idx: usize,
    show_suggestions: bool,
    validate: Option<Box<dyn Fn(&str) -> Result<(), String> + Send>>,
    validation_err: Option<String>,
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            key_map: TextInputKeyMap::default(),
            value: Vec::new(),
            pos: 0,
            offset: 0,
            offset_right: 0,
            width: 40,
            prompt: String::new(),
            placeholder: String::new(),
            echo_mode: EchoMode::Normal,
            echo_char: '*',
            focus: false,
            cursor: Cursor::new(),
            char_limit: 0, // 0 means no limit
            style: Style::new(),
            placeholder_style: Style::new().faint(true),
            cursor_style: Style::new(),
            suggestions: Vec::new(),
            matched_suggestions: Vec::new(),
            current_suggestion_idx: 0,
            show_suggestions: false,
            validate: None,
            validation_err: None,
        }
    }

    pub fn with_prompt(mut self, p: &str) -> Self {
        self.prompt = p.to_string();
        self
    }

    pub fn with_placeholder(mut self, p: &str) -> Self {
        self.placeholder = p.to_string();
        self
    }

    pub fn with_echo_mode(mut self, mode: EchoMode) -> Self {
        self.echo_mode = mode;
        self
    }

    pub fn with_char_limit(mut self, limit: usize) -> Self {
        self.char_limit = limit;
        self
    }

    pub fn with_width(mut self, w: usize) -> Self {
        self.width = w;
        self
    }

    pub fn set_suggestions(&mut self, suggestions: Vec<String>) {
        self.suggestions = suggestions;
        self.update_suggestions();
    }

    pub fn current_suggestion(&self) -> Option<&str> {
        if self.show_suggestions && !self.matched_suggestions.is_empty() {
            Some(&self.matched_suggestions[self.current_suggestion_idx])
        } else {
            None
        }
    }

    pub fn set_validate<F: Fn(&str) -> Result<(), String> + Send + 'static>(&mut self, f: F) {
        self.validate = Some(Box::new(f));
    }

    pub fn validation_error(&self) -> Option<&str> {
        self.validation_err.as_deref()
    }

    pub fn value(&self) -> String {
        self.value.iter().collect()
    }

    pub fn set_value(&mut self, s: &str) {
        self.value = s.chars().collect();
        if self.pos > self.value.len() {
            self.pos = self.value.len();
        }
        self.update_offset();
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.pos = pos.min(self.value.len());
        self.update_offset();
    }

    pub fn focus(&mut self) -> Cmd {
        self.focus = true;
        self.cursor.set_mode(CursorMode::Blink);
        self.cursor.focus()
    }

    pub fn blur(&mut self) {
        self.focus = false;
        self.cursor.blur();
    }

    pub fn focused(&self) -> bool {
        self.focus
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        // Forward to cursor first
        let cursor_cmd = self.cursor.update(msg);

        if !self.focus {
            return cursor_cmd;
        }

        let old_value: String = self.value.iter().collect();

        if let Msg::KeyPress(key) = msg {
            if self.key_map.delete_char_backward.matches(key) {
                self.delete_before_cursor();
            } else if self.key_map.delete_char_forward.matches(key) {
                self.delete_after_cursor();
            } else if self.key_map.word_forward.matches(key) {
                self.cursor_word_right();
            } else if self.key_map.word_backward.matches(key) {
                self.cursor_word_left();
            } else if self.key_map.char_backward.matches(key) {
                self.cursor_left();
            } else if self.key_map.char_forward.matches(key) {
                self.cursor_right();
            } else if self.key_map.line_start.matches(key) {
                self.cursor_start();
            } else if self.key_map.line_end.matches(key) {
                self.cursor_end();
            } else if self.key_map.delete_word_backward.matches(key) {
                self.delete_word_before_cursor();
            } else if self.key_map.delete_before_cursor.matches(key) {
                self.delete_to_start();
            } else if self.key_map.kill_line.matches(key) {
                self.delete_to_end();
            } else if let KeyCode::Char(ch) = key.code {
                // Insert normal characters (no modifiers except shift)
                if !key.modifiers.contains(Modifiers::CTRL) && !key.modifiers.contains(Modifiers::ALT) {
                    self.insert_char(ch);
                }
            }
            self.update_cursor_char();
            self.handle_overflow();

            // Update suggestions and validation when value changes
            let new_value: String = self.value.iter().collect();
            if old_value != new_value {
                self.update_suggestions();
                self.run_validation();
            }
        }

        cursor_cmd
    }

    pub fn view(&self) -> String {
        let mut out = String::new();

        // Prompt
        out.push_str(&self.prompt);

        if self.value.is_empty() && !self.placeholder.is_empty() && !self.focus {
            // Show placeholder
            out.push_str(&self.placeholder_style.render(&[&self.placeholder]));
            return out;
        }

        let display_chars = self.get_display_chars();
        let visible_width = self.available_width();

        // Get visible portion
        let visible_end = (self.offset + visible_width).min(display_chars.len());
        let visible: Vec<char> = display_chars[self.offset..visible_end].to_vec();

        // Render characters with cursor
        let cursor_visible_pos = self.pos.saturating_sub(self.offset);

        let cursor_style = Style::new().reverse(true);

        for (i, ch) in visible.iter().enumerate() {
            if i == cursor_visible_pos && self.focus {
                out.push_str(&cursor_style.render(&[&ch.to_string()]));
            } else {
                out.push_str(&self.style.render(&[&ch.to_string()]));
            }
        }

        // If cursor is at the end, show the cursor as a space
        if cursor_visible_pos >= visible.len() && self.focus {
            out.push_str(&cursor_style.render(&[" "]));
        }

        out
    }

    fn insert_char(&mut self, ch: char) {
        if self.char_limit > 0 && self.value.len() >= self.char_limit {
            return;
        }
        self.value.insert(self.pos, ch);
        self.pos += 1;
        self.update_offset();
    }

    fn delete_before_cursor(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
            self.value.remove(self.pos);
            self.update_offset();
        }
    }

    fn delete_after_cursor(&mut self) {
        if self.pos < self.value.len() {
            self.value.remove(self.pos);
        }
    }

    fn cursor_left(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
            self.update_offset();
        }
    }

    fn cursor_right(&mut self) {
        if self.pos < self.value.len() {
            self.pos += 1;
            self.update_offset();
        }
    }

    fn cursor_start(&mut self) {
        self.pos = 0;
        self.update_offset();
    }

    fn cursor_end(&mut self) {
        self.pos = self.value.len();
        self.update_offset();
    }

    fn cursor_word_right(&mut self) {
        let len = self.value.len();
        if self.pos >= len {
            return;
        }
        // Skip current word chars
        while self.pos < len && self.value[self.pos] != ' ' {
            self.pos += 1;
        }
        // Skip whitespace
        while self.pos < len && self.value[self.pos] == ' ' {
            self.pos += 1;
        }
        self.update_offset();
    }

    fn cursor_word_left(&mut self) {
        if self.pos == 0 {
            return;
        }
        // Skip whitespace
        while self.pos > 0 && self.value[self.pos - 1] == ' ' {
            self.pos -= 1;
        }
        // Skip word chars
        while self.pos > 0 && self.value[self.pos - 1] != ' ' {
            self.pos -= 1;
        }
        self.update_offset();
    }

    fn delete_word_before_cursor(&mut self) {
        if self.pos == 0 {
            return;
        }
        // Skip whitespace
        let mut end = self.pos;
        while end > 0 && self.value[end - 1] == ' ' {
            end -= 1;
        }
        // Skip word chars
        while end > 0 && self.value[end - 1] != ' ' {
            end -= 1;
        }
        self.value.drain(end..self.pos);
        self.pos = end;
        self.update_offset();
    }

    fn delete_to_start(&mut self) {
        self.value.drain(0..self.pos);
        self.pos = 0;
        self.update_offset();
    }

    fn delete_to_end(&mut self) {
        self.value.truncate(self.pos);
    }

    fn available_width(&self) -> usize {
        let prompt_width = rouge_ansi::string_width(&self.prompt);
        self.width.saturating_sub(prompt_width).saturating_sub(1) // -1 for cursor
    }

    fn update_offset(&mut self) {
        let avail = self.available_width();
        if avail == 0 {
            return;
        }
        if self.pos < self.offset {
            self.offset = self.pos;
        } else if self.pos >= self.offset + avail {
            self.offset = self.pos - avail + 1;
        }
    }

    fn get_display_chars(&self) -> Vec<char> {
        match self.echo_mode {
            EchoMode::Normal => self.value.clone(),
            EchoMode::Password => vec![self.echo_char; self.value.len()],
            EchoMode::None => Vec::new(),
        }
    }

    fn update_cursor_char(&mut self) {
        let display = self.get_display_chars();
        if self.pos < display.len() {
            self.cursor.set_char(&display[self.pos].to_string());
        } else {
            self.cursor.set_char(" ");
        }
    }

    /// Adjust horizontal scroll offset based on cursor position and width.
    fn handle_overflow(&mut self) {
        let avail = self.available_width();
        if avail == 0 {
            return;
        }
        if self.pos < self.offset {
            self.offset = self.pos;
            self.offset_right = self.offset + avail;
        } else if self.pos >= self.offset_right {
            self.offset_right = self.pos + 1;
            self.offset = self.offset_right.saturating_sub(avail);
        }
        // Clamp offset_right
        let max_len = self.value.len();
        if self.offset_right > max_len + 1 {
            self.offset_right = max_len + 1;
        }
    }

    fn update_suggestions(&mut self) {
        if self.suggestions.is_empty() {
            self.matched_suggestions.clear();
            self.show_suggestions = false;
            return;
        }
        let value: String = self.value.iter().collect();
        if value.is_empty() {
            self.matched_suggestions.clear();
            self.show_suggestions = false;
            return;
        }
        let lower_value = value.to_lowercase();
        self.matched_suggestions = self
            .suggestions
            .iter()
            .filter(|s| s.to_lowercase().starts_with(&lower_value))
            .cloned()
            .collect();
        self.show_suggestions = !self.matched_suggestions.is_empty();
        // Reset index if out of bounds
        if self.current_suggestion_idx >= self.matched_suggestions.len() {
            self.current_suggestion_idx = 0;
        }
    }

    fn run_validation(&mut self) {
        if let Some(ref validate) = self.validate {
            let value: String = self.value.iter().collect();
            match validate(&value) {
                Ok(()) => self.validation_err = None,
                Err(e) => self.validation_err = Some(e),
            }
        }
    }
}
