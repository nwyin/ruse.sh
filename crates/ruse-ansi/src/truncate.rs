use unicode_width::UnicodeWidthChar;

use crate::width::string_width;

/// Truncate a string to a given visual width from the right, preserving ANSI sequences.
///
/// If the string is longer than `width`, it is truncated and `tail` is appended.
/// The total visual width of the result (including `tail`) will not exceed `width`.
/// ANSI escape sequences within the kept portion are preserved.
pub fn truncate(s: &str, width: usize, tail: &str) -> String {
    let current_width = string_width(s);
    if current_width <= width {
        return s.to_string();
    }

    let tail_width = string_width(tail);
    if tail_width >= width {
        // Tail itself is wider than the target; just return truncated tail
        return tail[..width.min(tail.len())].to_string();
    }

    let target_width = width - tail_width;
    let mut out = String::new();
    let mut vis_width: usize = 0;
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Start of escape sequence — always include it
            out.push(ch);
            match chars.peek() {
                Some(&'[') => {
                    out.push(chars.next().unwrap());
                    for c in chars.by_ref() {
                        out.push(c);
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(&']') => {
                    out.push(chars.next().unwrap());
                    while let Some(c) = chars.next() {
                        match c {
                            '\x07' => {
                                out.push('\x07');
                                break;
                            }
                            '\x1b' => {
                                out.push('\x1b');
                                if chars.peek() == Some(&'\\') {
                                    out.push(chars.next().unwrap());
                                }
                                break;
                            }
                            _ => out.push(c),
                        }
                    }
                }
                Some(_) => {
                    if let Some(c) = chars.next() {
                        out.push(c);
                    }
                }
                None => {}
            }
        } else {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if vis_width + ch_width > target_width {
                break;
            }
            vis_width += ch_width;
            out.push(ch);
        }
    }

    out.push_str(tail);
    out
}

/// Truncate a string from the left, preserving ANSI sequences.
///
/// If the string is longer than `width`, leading characters are removed and
/// `prefix` is prepended. The total visual width of the result (including
/// `prefix`) will not exceed `width`.
pub fn truncate_left(s: &str, width: usize, prefix: &str) -> String {
    let current_width = string_width(s);
    if current_width <= width {
        return s.to_string();
    }

    let prefix_width = string_width(prefix);
    if prefix_width >= width {
        return prefix[..width.min(prefix.len())].to_string();
    }

    let target_width = width - prefix_width;

    // We need to collect characters and their widths, skipping ANSI sequences,
    // then figure out how many to skip from the front.
    // Strategy: collect all "segments" — either ANSI sequences or visible chars.
    let segments = parse_segments(s);

    // Calculate total visible width
    let total_vis: usize = segments
        .iter()
        .map(|seg| match seg {
            Segment::Visible(ch) => UnicodeWidthChar::width(*ch).unwrap_or(0),
            Segment::Ansi(_) => 0,
        })
        .sum();

    // We want to skip (total_vis - target_width) visible columns from the start
    let skip_width = total_vis.saturating_sub(target_width);

    let mut skipped: usize = 0;
    let mut out = String::from(prefix);
    let mut past_skip = false;

    for seg in &segments {
        match seg {
            Segment::Visible(ch) => {
                let w = UnicodeWidthChar::width(*ch).unwrap_or(0);
                if !past_skip {
                    skipped += w;
                    if skipped >= skip_width {
                        past_skip = true;
                        // Only include this char if we haven't overshot
                        if skipped == skip_width {
                            // Exactly at boundary, don't include this char
                            // (it was consumed by the skip)
                        } else {
                            // We crossed the boundary ON this char, include it
                            // only if the remaining after skip fits
                        }
                        // Actually, we need to exclude all chars up to skip_width.
                        // If skipped > skip_width after adding this char, the char
                        // straddles the boundary. For simplicity with wide chars,
                        // skip it too.
                    }
                } else {
                    out.push(*ch);
                }
            }
            Segment::Ansi(seq) => {
                if past_skip {
                    out.push_str(seq);
                }
            }
        }
    }

    out
}

#[derive(Debug)]
enum Segment {
    Visible(char),
    Ansi(String),
}

fn parse_segments(s: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            let mut seq = String::new();
            seq.push(ch);
            match chars.peek() {
                Some(&'[') => {
                    seq.push(chars.next().unwrap());
                    for c in chars.by_ref() {
                        seq.push(c);
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(&']') => {
                    seq.push(chars.next().unwrap());
                    while let Some(c) = chars.next() {
                        match c {
                            '\x07' => {
                                seq.push('\x07');
                                break;
                            }
                            '\x1b' => {
                                seq.push('\x1b');
                                if chars.peek() == Some(&'\\') {
                                    seq.push(chars.next().unwrap());
                                }
                                break;
                            }
                            _ => seq.push(c),
                        }
                    }
                }
                Some(_) => {
                    if let Some(c) = chars.next() {
                        seq.push(c);
                    }
                }
                None => {}
            }
            segments.push(Segment::Ansi(seq));
        } else {
            segments.push(Segment::Visible(ch));
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_truncation() {
        assert_eq!(truncate("hello", 10, "…"), "hello");
    }

    #[test]
    fn test_truncate_simple() {
        assert_eq!(truncate("hello world", 8, "…"), "hello w…");
    }

    #[test]
    fn test_truncate_exact() {
        assert_eq!(truncate("hello", 5, "…"), "hello");
    }

    #[test]
    fn test_truncate_with_ansi() {
        let s = "\x1b[1mhello world\x1b[0m";
        let result = truncate(s, 8, "…");
        // Should keep the bold sequence and truncate the visible text
        assert!(result.contains("\x1b[1m"));
        assert_eq!(string_width(&result), 8);
    }

    #[test]
    fn test_truncate_empty_tail() {
        assert_eq!(truncate("hello world", 5, ""), "hello");
    }

    #[test]
    fn test_truncate_zero_width() {
        assert_eq!(truncate("hello", 0, ""), "");
    }

    #[test]
    fn test_truncate_left_no_truncation() {
        assert_eq!(truncate_left("hello", 10, "…"), "hello");
    }

    #[test]
    fn test_truncate_left_simple() {
        let result = truncate_left("hello world", 8, "…");
        assert_eq!(result, "…o world");
    }

    #[test]
    fn test_truncate_left_empty_prefix() {
        let result = truncate_left("hello world", 5, "");
        assert_eq!(result, "world");
    }

    #[test]
    fn test_truncate_cjk() {
        // "你好世界" = 8 columns
        let result = truncate("你好世界", 5, "…");
        // "你好" = 4 cols + "…" = 1 col = 5
        assert_eq!(result, "你好…");
    }
}
