use std::collections::HashSet;

use ruse_ansi::strip_ansi;

use crate::style::Style;

/// A range of character indices to style.
#[derive(Debug, Clone)]
pub struct StyleRange {
    /// Start character index (inclusive).
    pub start: usize,
    /// End character index (exclusive).
    pub end: usize,
    /// Style to apply to this range.
    pub style: Style,
}

/// Apply different styles to specific character indices in a string.
///
/// Characters at positions in `indices` are rendered with `matched_style`,
/// all other characters are rendered with `unmatched_style`.
/// This is useful for fuzzy match highlighting.
///
/// Indexing is based on the ANSI-stripped version of `text` so that
/// any pre-existing escape sequences do not affect character positions.
pub fn style_runes(
    text: &str,
    indices: &[usize],
    matched_style: &Style,
    unmatched_style: &Style,
) -> String {
    let plain = strip_ansi(text);
    if plain.is_empty() {
        return String::new();
    }

    let match_set: HashSet<usize> = indices.iter().copied().collect();
    let matched_sgr = matched_style.text_sgr();
    let unmatched_sgr = unmatched_style.text_sgr();

    let chars: Vec<char> = plain.chars().collect();
    let mut result = String::with_capacity(text.len() + 32);

    // Group consecutive characters by whether they are matched
    let mut i = 0;
    while i < chars.len() {
        let is_match = match_set.contains(&i);
        let group_start = i;
        while i < chars.len() && match_set.contains(&i) == is_match {
            i += 1;
        }
        let segment: String = chars[group_start..i].iter().collect();
        let sgr = if is_match {
            &matched_sgr
        } else {
            &unmatched_sgr
        };
        result.push_str(&sgr.styled(&segment));
    }

    result
}

/// Apply different styles to specific character ranges in a string.
///
/// Each `StyleRange` specifies a `[start, end)` character range and the
/// style to apply. Characters not covered by any range are emitted unstyled.
///
/// Ranges must not overlap. They are sorted internally by `start`.
/// Indexing is based on the ANSI-stripped version of `text`.
pub fn style_ranges(text: &str, ranges: &[StyleRange]) -> String {
    let plain = strip_ansi(text);
    if plain.is_empty() {
        return String::new();
    }

    let chars: Vec<char> = plain.chars().collect();
    let len = chars.len();

    // Sort ranges by start position
    let mut sorted: Vec<&StyleRange> = ranges.iter().collect();
    sorted.sort_by_key(|r| r.start);

    let mut result = String::with_capacity(text.len() + 64);
    let mut pos = 0;

    for range in &sorted {
        let start = range.start.min(len);
        let end = range.end.min(len);

        // Emit unstyled gap before this range
        if pos < start {
            let gap: String = chars[pos..start].iter().collect();
            result.push_str(&gap);
        }

        // Emit the styled range
        if start < end {
            let segment: String = chars[start..end].iter().collect();
            let sgr = range.style.text_sgr();
            result.push_str(&sgr.styled(&segment));
        }

        pos = end;
    }

    // Emit any trailing unstyled text
    if pos < len {
        let tail: String = chars[pos..].iter().collect();
        result.push_str(&tail);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruse_colorprofile::Color;

    #[test]
    fn test_style_runes_some_indices() {
        let matched = Style::new().bold(true);
        let unmatched = Style::new();
        let result = style_runes("hello", &[1, 3], &matched, &unmatched);
        // 'h' unmatched, 'e' matched(bold), 'l' unmatched, 'l' matched(bold), 'o' unmatched
        assert!(result.contains("h"));
        assert!(result.contains("\x1b[1me\x1b[0m"));
        assert!(result.contains("\x1b[1ml\x1b[0m"));
        assert!(result.contains("o"));
    }

    #[test]
    fn test_style_runes_empty_indices() {
        let matched = Style::new().bold(true);
        let unmatched = Style::new().italic(true);
        let result = style_runes("abc", &[], &matched, &unmatched);
        // Everything should be unmatched (italic)
        assert_eq!(result, "\x1b[3mabc\x1b[0m");
    }

    #[test]
    fn test_style_runes_all_indices() {
        let matched = Style::new().bold(true);
        let unmatched = Style::new();
        let result = style_runes("ab", &[0, 1], &matched, &unmatched);
        // Everything matched (bold)
        assert_eq!(result, "\x1b[1mab\x1b[0m");
    }

    #[test]
    fn test_style_runes_empty_text() {
        let matched = Style::new().bold(true);
        let unmatched = Style::new();
        let result = style_runes("", &[0], &matched, &unmatched);
        assert_eq!(result, "");
    }

    #[test]
    fn test_style_runes_with_color() {
        let matched = Style::new().foreground(Color::Rgb { r: 255, g: 0, b: 0 });
        let unmatched = Style::new();
        let result = style_runes("hello", &[0, 4], &matched, &unmatched);
        // 'h' matched (red fg), 'ell' unmatched, 'o' matched (red fg)
        assert!(result.contains("\x1b[38;2;255;0;0mh\x1b[0m"));
        assert!(result.contains("ell"));
        assert!(result.contains("\x1b[38;2;255;0;0mo\x1b[0m"));
    }

    #[test]
    fn test_style_ranges_non_overlapping() {
        // "hello world" = h(0)e(1)l(2)l(3)o(4) (5)w(6)o(7)r(8)l(9)d(10)
        let ranges = vec![
            StyleRange {
                start: 0,
                end: 2,
                style: Style::new().bold(true),
            },
            StyleRange {
                start: 5,
                end: 7,
                style: Style::new().italic(true),
            },
        ];
        let result = style_ranges("hello world", &ranges);
        // "he" bold, "llo" unstyled (indices 2..5), " w" italic (indices 5..7), "orld" unstyled
        assert!(result.contains("\x1b[1mhe\x1b[0m"));
        assert!(result.contains("llo"));
        assert!(result.contains("\x1b[3m w\x1b[0m"));
        assert!(result.contains("orld"));
    }

    #[test]
    fn test_style_ranges_adjacent() {
        let ranges = vec![
            StyleRange {
                start: 0,
                end: 3,
                style: Style::new().bold(true),
            },
            StyleRange {
                start: 3,
                end: 6,
                style: Style::new().italic(true),
            },
        ];
        let result = style_ranges("abcdef", &ranges);
        // "abc" bold, "def" italic
        assert!(result.contains("\x1b[1mabc\x1b[0m"));
        assert!(result.contains("\x1b[3mdef\x1b[0m"));
    }

    #[test]
    fn test_style_ranges_empty() {
        let result = style_ranges("hello", &[]);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_style_ranges_trailing_text() {
        let ranges = vec![StyleRange {
            start: 0,
            end: 2,
            style: Style::new().bold(true),
        }];
        let result = style_ranges("hello", &ranges);
        assert!(result.contains("\x1b[1mhe\x1b[0m"));
        assert!(result.contains("llo"));
    }

    #[test]
    fn test_style_ranges_with_ansi_input() {
        // Input already has ANSI codes; indexing should be based on stripped text
        let input = "\x1b[31mhello\x1b[0m world";
        let ranges = vec![StyleRange {
            start: 0,
            end: 5,
            style: Style::new().bold(true),
        }];
        let result = style_ranges(input, &ranges);
        // "hello" (stripped) should be bold, " world" unstyled
        assert!(result.contains("\x1b[1mhello\x1b[0m"));
        assert!(result.contains(" world"));
    }
}
