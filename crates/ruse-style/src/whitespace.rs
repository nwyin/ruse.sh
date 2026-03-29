use crate::style::Style;

/// Configurable whitespace renderer with character cycling and style.
pub struct Whitespace {
    chars: Vec<char>,
    style: Style,
}

impl Whitespace {
    /// Create a whitespace renderer that fills with spaces.
    pub fn new() -> Self {
        Self {
            chars: vec![' '],
            style: Style::new(),
        }
    }

    /// Set the characters to cycle through when filling whitespace.
    pub fn with_chars(mut self, chars: &[char]) -> Self {
        if !chars.is_empty() {
            self.chars = chars.to_vec();
        }
        self
    }

    /// Set the style to apply to whitespace.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Render whitespace of the given visual width.
    pub fn render(&self, width: usize) -> String {
        if width == 0 {
            return String::new();
        }
        let mut out = String::with_capacity(width);
        let mut remaining = width;
        let mut i = 0;
        while remaining > 0 {
            let ch = self.chars[i % self.chars.len()];
            let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1).max(1);
            if ch_width <= remaining {
                out.push(ch);
                remaining -= ch_width;
            } else {
                // Wide char doesn't fit, fill with spaces
                for _ in 0..remaining {
                    out.push(' ');
                }
                remaining = 0;
            }
            i += 1;
        }
        self.style.render(&[&out])
    }
}

impl Default for Whitespace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_whitespace() {
        let ws = Whitespace::new();
        let result = ws.render(5);
        assert_eq!(ruse_ansi::string_width(&result), 5);
    }

    #[test]
    fn test_custom_chars() {
        let ws = Whitespace::new().with_chars(&['.', ' ']);
        let rendered = ws.render(6);
        let stripped = ruse_ansi::strip_ansi(&rendered);
        assert_eq!(stripped, ". . . ");
    }

    #[test]
    fn test_zero_width() {
        let ws = Whitespace::new();
        assert_eq!(ws.render(0), "");
    }
}
