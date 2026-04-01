use unicode_width::UnicodeWidthChar;

/// Hard-wrap a string at a given visual width, preserving ANSI sequences.
///
/// Characters are broken at exactly `width` columns. ANSI style state is
/// carried across line breaks (the current sequence is closed with ESC[0m
/// at the break and reopened on the next line).
pub fn wrap(s: &str, width: usize) -> String {
    if width == 0 {
        return s.to_string();
    }

    let mut out = String::new();
    let mut col: usize = 0;
    let mut active_seqs: Vec<String> = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\n' {
            out.push('\n');
            col = 0;
            continue;
        }

        if ch == '\x1b' {
            let seq = consume_escape(&mut chars, ch);
            track_sgr_sequence(&seq, &mut active_seqs);
            out.push_str(&seq);
            continue;
        }

        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);

        if col + ch_width > width {
            // Close active sequences, insert newline, reopen
            if !active_seqs.is_empty() {
                out.push_str("\x1b[0m");
            }
            out.push('\n');
            for seq in &active_seqs {
                out.push_str(seq);
            }
            col = 0;
        }

        out.push(ch);
        col += ch_width;
    }

    out
}

/// Word-wrap a string at a given visual width, preserving ANSI sequences.
///
/// Breaks at word boundaries (spaces) when possible, falling back to
/// hard-breaking if a single word exceeds the width. ANSI style state is
/// carried across line breaks.
pub fn wordwrap(s: &str, width: usize) -> String {
    if width == 0 {
        return s.to_string();
    }

    let mut result = String::new();

    for (line_idx, line) in s.split('\n').enumerate() {
        if line_idx > 0 {
            result.push('\n');
        }
        // Detect leading whitespace so continuation lines preserve indent
        let indent = line.chars().take_while(|&c| c == ' ').count();
        wordwrap_line(line, width, indent, &mut result);
    }

    result
}

fn wordwrap_line(line: &str, width: usize, hanging_indent: usize, out: &mut String) {
    // Parse the line into tokens: words, spaces, and ANSI sequences
    let tokens = tokenize(line);

    let mut col: usize = 0;
    let mut active_seqs: Vec<String> = Vec::new();
    let mut line_start = true;
    let mut is_first_line = true;

    for token in &tokens {
        match token {
            Token::Ansi(seq) => {
                track_sgr_sequence(seq, &mut active_seqs);
                out.push_str(seq);
            }
            Token::Space => {
                if col + 1 > width {
                    // Break before the space
                    if !active_seqs.is_empty() {
                        out.push_str("\x1b[0m");
                    }
                    out.push('\n');
                    for _ in 0..hanging_indent {
                        out.push(' ');
                    }
                    for seq in &active_seqs {
                        out.push_str(seq);
                    }
                    col = hanging_indent;
                    line_start = true;
                    is_first_line = false;
                } else if !line_start || is_first_line {
                    out.push(' ');
                    col += 1;
                }
            }
            Token::Word(word, word_width) => {
                if !line_start && col + word_width > width {
                    // Word doesn't fit on current line, break
                    if !active_seqs.is_empty() {
                        out.push_str("\x1b[0m");
                    }
                    out.push('\n');
                    for _ in 0..hanging_indent {
                        out.push(' ');
                    }
                    for seq in &active_seqs {
                        out.push_str(seq);
                    }
                    col = hanging_indent;
                    line_start = true;
                    is_first_line = false;
                }

                if *word_width <= width {
                    out.push_str(word);
                    col += word_width;
                    line_start = false;
                } else {
                    // Word is wider than width; hard-break it
                    let mut wchars = word.chars().peekable();
                    while let Some(wch) = wchars.next() {
                        if wch == '\x1b' {
                            let seq = consume_escape(&mut wchars, wch);
                            track_sgr_sequence(&seq, &mut active_seqs);
                            out.push_str(&seq);
                            continue;
                        }

                        let ch_w = UnicodeWidthChar::width(wch).unwrap_or(0);
                        if col + ch_w > width && col > 0 {
                            if !active_seqs.is_empty() {
                                out.push_str("\x1b[0m");
                            }
                            out.push('\n');
                            for _ in 0..hanging_indent {
                                out.push(' ');
                            }
                            for seq in &active_seqs {
                                out.push_str(seq);
                            }
                            col = hanging_indent;
                        }
                        out.push(wch);
                        col += ch_w;
                        line_start = false;
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
enum Token {
    Word(String, usize), // text, visual width
    Space,
    Ansi(String),
}

fn tokenize(s: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = s.chars().peekable();
    let mut current_word = String::new();
    let mut current_width: usize = 0;

    while let Some(&ch) = chars.peek() {
        if ch == '\x1b' {
            // If we're in a word, the ANSI sequence is part of the word
            chars.next();
            let seq = consume_escape(&mut chars, ch);
            if current_word.is_empty() {
                tokens.push(Token::Ansi(seq));
            } else {
                current_word.push_str(&seq);
            }
        } else if ch == ' ' {
            if !current_word.is_empty() {
                tokens.push(Token::Word(
                    std::mem::take(&mut current_word),
                    current_width,
                ));
                current_width = 0;
            }
            tokens.push(Token::Space);
            chars.next();
        } else {
            chars.next();
            current_word.push(ch);
            current_width += UnicodeWidthChar::width(ch).unwrap_or(0);
        }
    }

    if !current_word.is_empty() {
        tokens.push(Token::Word(current_word, current_width));
    }

    tokens
}

/// Consume an escape sequence starting after ESC has already been read.
fn consume_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, esc: char) -> String {
    let mut seq = String::new();
    seq.push(esc);

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

    seq
}

/// Track SGR sequences to know what's "active".
/// An ESC[0m resets everything. Other SGR sequences get added.
fn track_sgr_sequence(seq: &str, active: &mut Vec<String>) {
    // Only track CSI sequences ending with 'm' (SGR)
    if seq.starts_with("\x1b[") && seq.ends_with('m') {
        let params = &seq[2..seq.len() - 1];
        if params == "0" || params.is_empty() {
            active.clear();
        } else {
            active.push(seq.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strip::strip_ansi;

    #[test]
    fn test_wrap_no_break() {
        assert_eq!(wrap("hello", 10), "hello");
    }

    #[test]
    fn test_wrap_break() {
        assert_eq!(strip_ansi(&wrap("hello world", 5)), "hello\n worl\nd");
    }

    #[test]
    fn test_wrap_exact() {
        assert_eq!(wrap("hello", 5), "hello");
    }

    #[test]
    fn test_wrap_with_ansi() {
        let s = "\x1b[1mhello world\x1b[0m";
        let result = wrap(s, 5);
        // Should contain line break and re-opened bold
        assert!(result.contains('\n'));
        let stripped = strip_ansi(&result);
        assert_eq!(stripped, "hello\n worl\nd");
    }

    #[test]
    fn test_wordwrap_no_break() {
        assert_eq!(wordwrap("hello", 10), "hello");
    }

    #[test]
    fn test_wordwrap_break() {
        let result = wordwrap("hello world foo", 11);
        let stripped = strip_ansi(&result);
        assert_eq!(stripped, "hello world\nfoo");
    }

    #[test]
    fn test_wordwrap_long_word() {
        let result = wordwrap("superlongword", 5);
        let stripped = strip_ansi(&result);
        // Should hard-break at exactly width columns
        assert_eq!(stripped, "super\nlongw\nord");
        assert!(stripped.lines().all(|l| crate::width::string_width(l) <= 5));
    }

    #[test]
    fn test_wordwrap_preserves_existing_newlines() {
        let result = wordwrap("hello\nworld", 20);
        assert_eq!(result, "hello\nworld");
    }

    #[test]
    fn test_wordwrap_with_ansi() {
        let s = "\x1b[31mhello world\x1b[0m";
        let result = wordwrap(s, 5);
        let stripped = strip_ansi(&result);
        assert_eq!(stripped, "hello\nworld");
    }

    #[test]
    fn test_wrap_preserves_newlines() {
        assert_eq!(wrap("ab\ncd", 5), "ab\ncd");
    }

    #[test]
    fn test_wrap_zero_width() {
        assert_eq!(wrap("hello", 0), "hello");
    }

    #[test]
    fn test_wordwrap_preserves_indent() {
        // Simulates a bullet list item with 2-space margin
        let result = wordwrap("  hello world foo bar baz", 16);
        let stripped = strip_ansi(&result);
        // First line should have content
        let lines: Vec<&str> = stripped.lines().collect();
        assert!(
            lines[0].starts_with("  hello"),
            "first line: {:?}",
            lines[0]
        );
        // Continuation lines should preserve the 2-space indent
        for line in &lines[1..] {
            assert!(
                line.starts_with("  "),
                "continuation line should be indented: {:?}",
                line
            );
        }
    }

    #[test]
    fn test_wordwrap_indent_no_overflow() {
        let result = wordwrap("    short", 20);
        assert_eq!(strip_ansi(&result), "    short");
    }

    #[test]
    fn test_wordwrap_indent_with_ansi() {
        let s = "  \x1b[1mhello world foo bar\x1b[0m";
        let result = wordwrap(s, 14);
        let stripped = strip_ansi(&result);
        for line in stripped.lines() {
            assert!(
                line.starts_with("  "),
                "all lines should be indented: {:?}",
                line
            );
        }
    }
}
