/// Border contains the characters used to draw a border around a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Border {
    pub top: &'static str,
    pub bottom: &'static str,
    pub left: &'static str,
    pub right: &'static str,
    pub top_left: &'static str,
    pub top_right: &'static str,
    pub bottom_left: &'static str,
    pub bottom_right: &'static str,
    pub middle_left: &'static str,
    pub middle_right: &'static str,
    pub middle: &'static str,
    pub middle_top: &'static str,
    pub middle_bottom: &'static str,
}

impl Default for Border {
    fn default() -> Self {
        NO_BORDER
    }
}

/// An empty border (no characters).
pub const NO_BORDER: Border = Border {
    top: "",
    bottom: "",
    left: "",
    right: "",
    top_left: "",
    top_right: "",
    bottom_left: "",
    bottom_right: "",
    middle_left: "",
    middle_right: "",
    middle: "",
    middle_top: "",
    middle_bottom: "",
};

/// Standard box-drawing border with 90-degree corners.
pub const NORMAL_BORDER: Border = Border {
    top: "\u{2500}",           // ─
    bottom: "\u{2500}",        // ─
    left: "\u{2502}",          // │
    right: "\u{2502}",         // │
    top_left: "\u{250C}",      // ┌
    top_right: "\u{2510}",     // ┐
    bottom_left: "\u{2514}",   // └
    bottom_right: "\u{2518}",  // ┘
    middle_left: "\u{251C}",   // ├
    middle_right: "\u{2524}",  // ┤
    middle: "\u{253C}",        // ┼
    middle_top: "\u{252C}",    // ┬
    middle_bottom: "\u{2534}", // ┴
};

/// Border with rounded corners.
pub const ROUNDED_BORDER: Border = Border {
    top: "\u{2500}",           // ─
    bottom: "\u{2500}",        // ─
    left: "\u{2502}",          // │
    right: "\u{2502}",         // │
    top_left: "\u{256D}",      // ╭
    top_right: "\u{256E}",     // ╮
    bottom_left: "\u{2570}",   // ╰
    bottom_right: "\u{256F}",  // ╯
    middle_left: "\u{251C}",   // ├
    middle_right: "\u{2524}",  // ┤
    middle: "\u{253C}",        // ┼
    middle_top: "\u{252C}",    // ┬
    middle_bottom: "\u{2534}", // ┴
};

/// Full block border.
pub const BLOCK_BORDER: Border = Border {
    top: "\u{2588}",           // █
    bottom: "\u{2588}",        // █
    left: "\u{2588}",          // █
    right: "\u{2588}",         // █
    top_left: "\u{2588}",      // █
    top_right: "\u{2588}",     // █
    bottom_left: "\u{2588}",   // █
    bottom_right: "\u{2588}",  // █
    middle_left: "\u{2588}",   // █
    middle_right: "\u{2588}",  // █
    middle: "\u{2588}",        // █
    middle_top: "\u{2588}",    // █
    middle_bottom: "\u{2588}", // █
};

/// Thick box-drawing border.
pub const THICK_BORDER: Border = Border {
    top: "\u{2501}",           // ━
    bottom: "\u{2501}",        // ━
    left: "\u{2503}",          // ┃
    right: "\u{2503}",         // ┃
    top_left: "\u{250F}",      // ┏
    top_right: "\u{2513}",     // ┓
    bottom_left: "\u{2517}",   // ┗
    bottom_right: "\u{251B}",  // ┛
    middle_left: "\u{2523}",   // ┣
    middle_right: "\u{252B}",  // ┫
    middle: "\u{254B}",        // ╋
    middle_top: "\u{2533}",    // ┳
    middle_bottom: "\u{253B}", // ┻
};

/// Double-line border.
pub const DOUBLE_BORDER: Border = Border {
    top: "\u{2550}",           // ═
    bottom: "\u{2550}",        // ═
    left: "\u{2551}",          // ║
    right: "\u{2551}",         // ║
    top_left: "\u{2554}",      // ╔
    top_right: "\u{2557}",     // ╗
    bottom_left: "\u{255A}",   // ╚
    bottom_right: "\u{255D}",  // ╝
    middle_left: "\u{2560}",   // ╠
    middle_right: "\u{2563}",  // ╣
    middle: "\u{256C}",        // ╬
    middle_top: "\u{2566}",    // ╦
    middle_bottom: "\u{2569}", // ╩
};

/// Hidden border (spaces, preserves layout).
pub const HIDDEN_BORDER: Border = Border {
    top: " ",
    bottom: " ",
    left: " ",
    right: " ",
    top_left: " ",
    top_right: " ",
    bottom_left: " ",
    bottom_right: " ",
    middle_left: " ",
    middle_right: " ",
    middle: " ",
    middle_top: " ",
    middle_bottom: " ",
};

/// ASCII border using `+`, `-`, and `|`.
pub const ASCII_BORDER: Border = Border {
    top: "-",
    bottom: "-",
    left: "|",
    right: "|",
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    middle_left: "+",
    middle_right: "+",
    middle: "+",
    middle_top: "+",
    middle_bottom: "+",
};

/// Markdown-style table border.
pub const MARKDOWN_BORDER: Border = Border {
    top: "-",
    bottom: "-",
    left: "|",
    right: "|",
    top_left: "|",
    top_right: "|",
    bottom_left: "|",
    bottom_right: "|",
    middle_left: "|",
    middle_right: "|",
    middle: "|",
    middle_top: "|",
    middle_bottom: "|",
};

/// Half-block border that sits outside the content frame.
pub const OUTER_HALF_BLOCK_BORDER: Border = Border {
    top: "\u{2580}",          // ▀
    bottom: "\u{2584}",       // ▄
    left: "\u{258C}",         // ▌
    right: "\u{2590}",        // ▐
    top_left: "\u{259B}",     // ▛
    top_right: "\u{259C}",    // ▜
    bottom_left: "\u{2599}",  // ▙
    bottom_right: "\u{259F}", // ▟
    middle_left: "",
    middle_right: "",
    middle: "",
    middle_top: "",
    middle_bottom: "",
};

/// Half-block border that sits inside the content frame.
pub const INNER_HALF_BLOCK_BORDER: Border = Border {
    top: "\u{2584}",          // ▄
    bottom: "\u{2580}",       // ▀
    left: "\u{2590}",         // ▐
    right: "\u{258C}",        // ▌
    top_left: "\u{2597}",     // ▗
    top_right: "\u{2596}",    // ▖
    bottom_left: "\u{259D}",  // ▝
    bottom_right: "\u{2598}", // ▘
    middle_left: "",
    middle_right: "",
    middle: "",
    middle_top: "",
    middle_bottom: "",
};

impl Border {
    /// Returns 1 if any part of this border edge has content, 0 otherwise.
    pub fn get_top_size(&self) -> usize {
        if self.top.is_empty() && self.top_left.is_empty() && self.top_right.is_empty() {
            0
        } else {
            1
        }
    }

    /// Returns 1 if any part of this border edge has content, 0 otherwise.
    pub fn get_bottom_size(&self) -> usize {
        if self.bottom.is_empty() && self.bottom_left.is_empty() && self.bottom_right.is_empty() {
            0
        } else {
            1
        }
    }

    /// Returns the visual width of the left border characters (max 1).
    pub fn get_left_size(&self) -> usize {
        max_char_width(&[self.top_left, self.left, self.bottom_left])
    }

    /// Returns the visual width of the right border characters (max 1).
    pub fn get_right_size(&self) -> usize {
        max_char_width(&[self.top_right, self.right, self.bottom_right])
    }
}

fn max_char_width(parts: &[&str]) -> usize {
    let mut max_w = 0;
    for part in parts {
        if !part.is_empty() {
            let w = ruse_ansi::string_width(part);
            if w > max_w {
                max_w = w;
            }
        }
    }
    max_w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_border_sizes() {
        assert_eq!(NORMAL_BORDER.get_top_size(), 1);
        assert_eq!(NORMAL_BORDER.get_bottom_size(), 1);
        assert_eq!(NORMAL_BORDER.get_left_size(), 1);
        assert_eq!(NORMAL_BORDER.get_right_size(), 1);
    }

    #[test]
    fn test_no_border_sizes() {
        assert_eq!(NO_BORDER.get_top_size(), 0);
        assert_eq!(NO_BORDER.get_bottom_size(), 0);
        assert_eq!(NO_BORDER.get_left_size(), 0);
        assert_eq!(NO_BORDER.get_right_size(), 0);
    }

    #[test]
    fn test_border_equality() {
        assert_eq!(NORMAL_BORDER, NORMAL_BORDER);
        assert_ne!(NORMAL_BORDER, ROUNDED_BORDER);
    }

    #[test]
    fn test_default_is_no_border() {
        assert_eq!(Border::default(), NO_BORDER);
    }
}
