use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::strip::strip_ansi;

/// Calculate the visual width of a string, ignoring ANSI escape sequences.
///
/// Uses grapheme cluster segmentation for correct handling of emoji sequences
/// (e.g., ZWJ sequences like 👨‍👩‍👧) and `unicode_width` for accurate
/// width calculation (e.g., CJK characters count as 2 columns).
pub fn string_width(s: &str) -> usize {
    let stripped = strip_ansi(s);
    stripped
        .lines()
        .map(|line| grapheme_width(line))
        .max()
        .unwrap_or(0)
}

/// Calculate visual width of a single line using grapheme clusters.
pub fn grapheme_width(s: &str) -> usize {
    s.graphemes(true)
        .map(|g| UnicodeWidthStr::width(g))
        .sum()
}

/// Count the number of visual lines in a string.
///
/// Returns the number of newlines + 1. An empty string returns 1.
pub fn string_height(s: &str) -> usize {
    if s.is_empty() {
        return 1;
    }
    s.chars().filter(|&c| c == '\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii() {
        assert_eq!(string_width("hello"), 5);
    }

    #[test]
    fn test_with_ansi() {
        assert_eq!(string_width("\x1b[1mhello\x1b[0m"), 5);
    }

    #[test]
    fn test_cjk() {
        // CJK characters are double-width
        assert_eq!(string_width("你好"), 4);
    }

    #[test]
    fn test_mixed_cjk_ansi() {
        assert_eq!(string_width("\x1b[31m你好\x1b[0m world"), 10);
    }

    #[test]
    fn test_empty() {
        assert_eq!(string_width(""), 0);
    }

    #[test]
    fn test_multiline_width() {
        assert_eq!(string_width("hello\nworld!"), 6);
    }

    #[test]
    fn test_height_single_line() {
        assert_eq!(string_height("hello"), 1);
    }

    #[test]
    fn test_height_empty() {
        assert_eq!(string_height(""), 1);
    }

    #[test]
    fn test_height_multiline() {
        assert_eq!(string_height("a\nb\nc"), 3);
    }

    #[test]
    fn test_height_trailing_newline() {
        assert_eq!(string_height("a\n"), 2);
    }
}
