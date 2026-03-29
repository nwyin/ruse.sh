use ruse_ansi::string_width;

use crate::position::Position;
use crate::render::get_lines;

/// Return the visual width of a (possibly multi-line) string.
///
/// Returns the width of the widest line.
pub fn width(s: &str) -> usize {
    string_width(s)
}

/// Return the number of lines in a string.
pub fn height(s: &str) -> usize {
    if s.is_empty() {
        return 1;
    }
    s.chars().filter(|&c| c == '\n').count() + 1
}

/// Join multiple strings horizontally, aligning them along a vertical axis.
///
/// `pos` controls vertical alignment: `Position::TOP` aligns tops,
/// `Position::BOTTOM` aligns bottoms, `Position::CENTER` centers.
pub fn join_horizontal(pos: Position, strs: &[&str]) -> String {
    if strs.is_empty() {
        return String::new();
    }
    if strs.len() == 1 {
        return strs[0].to_string();
    }

    let mut blocks: Vec<Vec<&str>> = Vec::with_capacity(strs.len());
    let mut max_widths: Vec<usize> = Vec::with_capacity(strs.len());
    let mut max_height = 0usize;

    for s in strs {
        let (lines, w) = get_lines(s);
        if lines.len() > max_height {
            max_height = lines.len();
        }
        max_widths.push(w);
        blocks.push(lines);
    }

    // Pad blocks to equal height
    for (i, block) in blocks.iter_mut().enumerate() {
        let _ = i;
        if block.len() >= max_height {
            continue;
        }

        let extra = max_height - block.len();

        if pos == Position::TOP {
            block.extend(std::iter::repeat_n("", extra));
        } else if pos == Position::BOTTOM {
            let mut new_block: Vec<&str> = vec![""; extra];
            new_block.append(block);
            *block = new_block;
        } else {
            // Somewhere in the middle
            let split = ((extra as f64) * pos.value()).round() as usize;
            let top = extra - split;
            let bottom = extra - top;

            let mut new_block: Vec<&str> = vec![""; top];
            new_block.append(block);
            new_block.extend(std::iter::repeat_n("", bottom));
            *block = new_block;
        }
    }

    // Merge lines side by side
    let mut out = String::new();
    for i in 0..max_height {
        for (j, block) in blocks.iter().enumerate() {
            let line = block[i];
            out.push_str(line);
            // Pad to max width for this column
            let line_w = string_width(line);
            let pad = max_widths[j].saturating_sub(line_w);
            if pad > 0 {
                out.push_str(&" ".repeat(pad));
            }
        }
        if i < max_height - 1 {
            out.push('\n');
        }
    }

    out
}

/// Join multiple strings vertically, aligning them along a horizontal axis.
///
/// `pos` controls horizontal alignment: `Position::LEFT` aligns left,
/// `Position::RIGHT` aligns right, `Position::CENTER` centers.
pub fn join_vertical(pos: Position, strs: &[&str]) -> String {
    if strs.is_empty() {
        return String::new();
    }
    if strs.len() == 1 {
        return strs[0].to_string();
    }

    let mut blocks: Vec<Vec<&str>> = Vec::with_capacity(strs.len());
    let mut max_width = 0usize;

    for s in strs {
        let (lines, w) = get_lines(s);
        if w > max_width {
            max_width = w;
        }
        blocks.push(lines);
    }

    let mut out = String::new();
    for (i, block) in blocks.iter().enumerate() {
        for (j, line) in block.iter().enumerate() {
            let w = max_width.saturating_sub(string_width(line));

            if pos == Position::LEFT {
                out.push_str(line);
                if w > 0 {
                    out.push_str(&" ".repeat(w));
                }
            } else if pos == Position::RIGHT {
                if w > 0 {
                    out.push_str(&" ".repeat(w));
                }
                out.push_str(line);
            } else {
                // Center
                if w < 1 {
                    out.push_str(line);
                } else {
                    let split = ((w as f64) * pos.value()).round() as usize;
                    let right = w - split;
                    let left = w - right;
                    out.push_str(&" ".repeat(left));
                    out.push_str(line);
                    out.push_str(&" ".repeat(right));
                }
            }

            // Newline unless last line of last block
            if !(i == blocks.len() - 1 && j == block.len() - 1) {
                out.push('\n');
            }
        }
    }

    out
}

/// Place a string in a box of the given dimensions.
pub fn place(width: usize, height: usize, h: Position, v: Position, s: &str) -> String {
    place_vertical(height, v, &place_horizontal(width, h, s))
}

/// Place a string horizontally in a space of the given width.
pub fn place_horizontal(target_width: usize, pos: Position, s: &str) -> String {
    let (lines, content_width) = get_lines(s);
    let gap = target_width.saturating_sub(content_width);

    if gap == 0 {
        return s.to_string();
    }

    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        let short = content_width.saturating_sub(string_width(line));
        let total_gap = gap + short;

        if pos == Position::LEFT {
            out.push_str(line);
            out.push_str(&" ".repeat(total_gap));
        } else if pos == Position::RIGHT {
            out.push_str(&" ".repeat(total_gap));
            out.push_str(line);
        } else {
            let split = ((total_gap as f64) * pos.value()).round() as usize;
            let left = total_gap - split;
            let right = total_gap - left;
            out.push_str(&" ".repeat(left));
            out.push_str(line);
            out.push_str(&" ".repeat(right));
        }

        if i < lines.len() - 1 {
            out.push('\n');
        }
    }

    out
}

/// Place a string vertically in a space of the given height.
pub fn place_vertical(target_height: usize, pos: Position, s: &str) -> String {
    let content_height = s.chars().filter(|&c| c == '\n').count() + 1;
    let gap = target_height.saturating_sub(content_height);

    if gap == 0 {
        return s.to_string();
    }

    let (_, w) = get_lines(s);
    let empty_line = " ".repeat(w);

    let mut out = String::new();

    if pos == Position::TOP {
        out.push_str(s);
        out.push('\n');
        for i in 0..gap {
            out.push_str(&empty_line);
            if i < gap - 1 {
                out.push('\n');
            }
        }
    } else if pos == Position::BOTTOM {
        for _ in 0..gap {
            out.push_str(&empty_line);
            out.push('\n');
        }
        out.push_str(s);
    } else {
        // Center
        let split = ((gap as f64) * pos.value()).round() as usize;
        let top = gap - split;
        let bottom = gap - top;

        for _ in 0..top {
            out.push_str(&empty_line);
            out.push('\n');
        }
        out.push_str(s);
        for _ in 0..bottom {
            out.push('\n');
            out.push_str(&empty_line);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_width_single_line() {
        assert_eq!(width("hello"), 5);
    }

    #[test]
    fn test_width_multiline() {
        assert_eq!(width("hello\nworld!"), 6);
    }

    #[test]
    fn test_height_single() {
        assert_eq!(height("hello"), 1);
    }

    #[test]
    fn test_height_multi() {
        assert_eq!(height("a\nb\nc"), 3);
    }

    #[test]
    fn test_height_empty() {
        assert_eq!(height(""), 1);
    }

    #[test]
    fn test_join_horizontal_top() {
        let a = "AAA\nAAA\nAAA";
        let b = "BB\nBB";
        let result = join_horizontal(Position::TOP, &[a, b]);
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "AAABB");
        assert_eq!(lines[1], "AAABB");
        assert_eq!(lines[2], "AAA  "); // b's block is padded with spaces
    }

    #[test]
    fn test_join_horizontal_bottom() {
        let a = "AAA\nAAA\nAAA";
        let b = "BB\nBB";
        let result = join_horizontal(Position::BOTTOM, &[a, b]);
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "AAA  ");
        assert_eq!(lines[1], "AAABB");
        assert_eq!(lines[2], "AAABB");
    }

    #[test]
    fn test_join_vertical_left() {
        let result = join_vertical(Position::LEFT, &["short", "a longer line"]);
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines[0], "short        ");
        assert_eq!(lines[1], "a longer line");
    }

    #[test]
    fn test_join_vertical_right() {
        let result = join_vertical(Position::RIGHT, &["short", "a longer line"]);
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines[0], "        short");
        assert_eq!(lines[1], "a longer line");
    }

    #[test]
    fn test_place_horizontal() {
        let result = place_horizontal(10, Position::CENTER, "hi");
        assert_eq!(string_width(&result), 10);
    }

    #[test]
    fn test_place_vertical() {
        let result = place_vertical(5, Position::TOP, "hi");
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], "hi");
    }

    #[test]
    fn test_place() {
        let result = place(10, 5, Position::CENTER, Position::CENTER, "hi");
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines.len(), 5);
        for line in &lines {
            assert_eq!(string_width(line), 10);
        }
    }

    #[test]
    fn test_join_empty() {
        assert_eq!(join_horizontal(Position::TOP, &[]), "");
        assert_eq!(join_vertical(Position::LEFT, &[]), "");
    }

    #[test]
    fn test_join_single() {
        assert_eq!(join_horizontal(Position::TOP, &["hello"]), "hello");
        assert_eq!(join_vertical(Position::LEFT, &["hello"]), "hello");
    }
}
