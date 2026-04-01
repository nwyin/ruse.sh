use unicode_width::UnicodeWidthChar;

use super::link::Link;
use super::style::CellStyle;

/// A single cell in the terminal grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The primary character.
    pub rune: char,
    /// Combining marks (zero-width diacritics).
    pub comb: Vec<char>,
    /// Display width in columns (1-4, or 0 for wide-cell placeholder).
    pub width: u8,
    /// Cell style (colors + attributes).
    pub style: CellStyle,
    /// Hyperlink metadata.
    pub link: Link,
}

impl Cell {
    /// Create a new cell from a character, auto-calculating width.
    pub fn new(ch: char) -> Self {
        let width = UnicodeWidthChar::width(ch).unwrap_or(0) as u8;
        Self {
            rune: ch,
            comb: Vec::new(),
            width,
            style: CellStyle::default(),
            link: Link::default(),
        }
    }

    /// Create a cell from a character with combining marks.
    pub fn new_with_comb(ch: char, comb: &[char]) -> Self {
        let width = UnicodeWidthChar::width(ch).unwrap_or(0) as u8;
        Self {
            rune: ch,
            comb: comb.to_vec(),
            width,
            style: CellStyle::default(),
            link: Link::default(),
        }
    }

    /// Create a blank cell (space, width 1).
    pub fn blank() -> Self {
        Self {
            rune: ' ',
            comb: Vec::new(),
            width: 1,
            style: CellStyle::default(),
            link: Link::default(),
        }
    }

    /// Create an empty placeholder cell (width 0).
    /// Used as continuation for wide characters.
    pub fn empty() -> Self {
        Self {
            rune: '\0',
            comb: Vec::new(),
            width: 0,
            style: CellStyle::default(),
            link: Link::default(),
        }
    }

    /// Whether this is an empty placeholder cell.
    pub fn is_empty(&self) -> bool {
        self.width == 0 && self.rune == '\0' && self.comb.is_empty()
    }

    /// Whether this cell is a blank space with default style.
    pub fn is_blank(&self) -> bool {
        self.rune == ' '
            && self.width == 1
            && self.comb.is_empty()
            && self.style.is_empty()
            && self.link.is_empty()
    }

    /// Whether this cell can be treated as visually clear
    /// (blank content + no visible style effects).
    pub fn is_clear(&self) -> bool {
        (self.is_blank() || self.is_empty()) && self.style.is_clear()
    }

    /// Reset to a blank space.
    pub fn make_blank(&mut self) {
        self.rune = ' ';
        self.comb.clear();
        self.width = 1;
        self.style = CellStyle::default();
        self.link = Link::default();
    }

    /// Append combining characters.
    pub fn append_comb(&mut self, chars: &[char]) {
        self.comb.extend_from_slice(chars);
    }

    /// Get the cell content as a String.
    pub fn content(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        let mut s = String::with_capacity(self.comb.len() + 1);
        s.push(self.rune);
        for &c in &self.comb {
            s.push(c);
        }
        s
    }

    /// Set style on the cell, returning self for chaining.
    pub fn with_style(mut self, style: CellStyle) -> Self {
        self.style = style;
        self
    }

    /// Set link on the cell, returning self for chaining.
    pub fn with_link(mut self, link: Link) -> Self {
        self.link = link;
        self
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::blank()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_cell() {
        let c = Cell::new('A');
        assert_eq!(c.rune, 'A');
        assert_eq!(c.width, 1);
        assert!(!c.is_empty());
        assert!(!c.is_blank());
    }

    #[test]
    fn test_cjk_cell() {
        let c = Cell::new('中');
        assert_eq!(c.width, 2);
        assert_eq!(c.content(), "中");
    }

    #[test]
    fn test_blank_cell() {
        let c = Cell::blank();
        assert!(c.is_blank());
        assert!(c.is_clear());
        assert_eq!(c.content(), " ");
    }

    #[test]
    fn test_empty_cell() {
        let c = Cell::empty();
        assert!(c.is_empty());
        assert!(c.is_clear());
        assert_eq!(c.content(), "");
    }

    #[test]
    fn test_combining() {
        let c = Cell::new_with_comb('e', &['\u{0301}']); // e + combining acute
        assert_eq!(c.width, 1);
        assert_eq!(c.content(), "e\u{0301}");
        assert_eq!(c.comb.len(), 1);
    }
}
