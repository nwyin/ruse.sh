/// Remove all ANSI escape sequences from a string.
///
/// Handles CSI sequences (ESC[...X), OSC sequences (ESC]...BEL/ST),
/// and other two-byte escape sequences.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            match chars.peek() {
                Some('[') => {
                    // CSI sequence: ESC[ followed by params until a byte in 0x40-0x7E
                    chars.next(); // consume '['
                    loop {
                        match chars.next() {
                            Some(c) if ('@'..='~').contains(&c) => break,
                            Some(_) => continue,
                            None => break,
                        }
                    }
                }
                Some(']') => {
                    // OSC sequence: ESC] ... terminated by BEL (0x07) or ST (ESC\)
                    chars.next(); // consume ']'
                    loop {
                        match chars.next() {
                            Some('\x07') => break,
                            Some('\x1b') => {
                                // Check for ST = ESC backslash
                                if chars.peek() == Some(&'\\') {
                                    chars.next();
                                }
                                break;
                            }
                            Some(_) => continue,
                            None => break,
                        }
                    }
                }
                Some(_) => {
                    // Other escape sequences: consume one more character
                    chars.next();
                }
                None => {}
            }
        } else {
            out.push(ch);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_ansi() {
        assert_eq!(strip_ansi("hello world"), "hello world");
    }

    #[test]
    fn test_sgr_bold() {
        assert_eq!(strip_ansi("\x1b[1mhello\x1b[0m"), "hello");
    }

    #[test]
    fn test_sgr_complex() {
        assert_eq!(strip_ansi("\x1b[38;5;196mred\x1b[0m text"), "red text");
    }

    #[test]
    fn test_cursor_movement() {
        assert_eq!(strip_ansi("\x1b[2;5Hhello"), "hello");
    }

    #[test]
    fn test_osc_bel() {
        assert_eq!(strip_ansi("\x1b]0;title\x07hello"), "hello");
    }

    #[test]
    fn test_osc_st() {
        assert_eq!(strip_ansi("\x1b]0;title\x1b\\hello"), "hello");
    }

    #[test]
    fn test_mixed() {
        let s = "\x1b[1m\x1b[31mhello\x1b[0m \x1b[4mworld\x1b[0m";
        assert_eq!(strip_ansi(s), "hello world");
    }

    #[test]
    fn test_empty() {
        assert_eq!(strip_ansi(""), "");
    }

    #[test]
    fn test_only_ansi() {
        assert_eq!(strip_ansi("\x1b[1m\x1b[0m"), "");
    }

    #[test]
    fn test_embedded_newlines() {
        assert_eq!(strip_ansi("\x1b[1mline1\n\x1b[0mline2"), "line1\nline2");
    }
}
