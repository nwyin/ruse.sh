use crate::width::string_width;

/// Pad each line of a string on the right to reach the given width with spaces.
///
/// If a line is already at least `width` columns wide, it is left unchanged.
pub fn pad(s: &str, width: usize) -> String {
    pad_right(s, width)
}

/// Pad each line of a string on the right to reach the given width with spaces.
///
/// If a line is already at least `width` columns wide, it is left unchanged.
pub fn pad_right(s: &str, width: usize) -> String {
    let lines: Vec<&str> = s.split('\n').collect();
    let mut out = String::new();

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(line);
        let line_width = string_width(line);
        if line_width < width {
            for _ in 0..(width - line_width) {
                out.push(' ');
            }
        }
    }

    out
}

/// Pad each line of a string on the left to reach the given width with spaces.
///
/// If a line is already at least `width` columns wide, it is left unchanged.
pub fn pad_left(s: &str, width: usize) -> String {
    let lines: Vec<&str> = s.split('\n').collect();
    let mut out = String::new();

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let line_width = string_width(line);
        if line_width < width {
            for _ in 0..(width - line_width) {
                out.push(' ');
            }
        }
        out.push_str(line);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_right_simple() {
        assert_eq!(pad("hi", 5), "hi   ");
    }

    #[test]
    fn test_pad_right_exact() {
        assert_eq!(pad("hello", 5), "hello");
    }

    #[test]
    fn test_pad_right_longer() {
        assert_eq!(pad("hello!", 5), "hello!");
    }

    #[test]
    fn test_pad_right_multiline() {
        assert_eq!(pad("hi\nbye", 5), "hi   \nbye  ");
    }

    #[test]
    fn test_pad_left_simple() {
        assert_eq!(pad_left("hi", 5), "   hi");
    }

    #[test]
    fn test_pad_left_exact() {
        assert_eq!(pad_left("hello", 5), "hello");
    }

    #[test]
    fn test_pad_left_longer() {
        assert_eq!(pad_left("hello!", 5), "hello!");
    }

    #[test]
    fn test_pad_left_multiline() {
        assert_eq!(pad_left("hi\nbye", 5), "   hi\n  bye");
    }

    #[test]
    fn test_pad_with_ansi() {
        let s = "\x1b[1mhi\x1b[0m";
        let result = pad(s, 5);
        // ANSI doesn't count for width, so 2 visible chars + 3 spaces
        assert_eq!(string_width(&result), 5);
    }

    #[test]
    fn test_pad_empty() {
        assert_eq!(pad("", 3), "   ");
    }

    #[test]
    fn test_pad_right_alias() {
        assert_eq!(pad("hi", 5), pad_right("hi", 5));
    }
}
