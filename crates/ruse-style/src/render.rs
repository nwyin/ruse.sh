use ruse_ansi::{SgrStyle, string_width, truncate, wordwrap};
use ruse_colorprofile::Color;

use crate::border::NO_BORDER;
use crate::position::Position;
use crate::style::{Props, Style, UnderlineStyle};

impl Style {
    /// Build the text-level SGR style from this Style (colors + text attributes only).
    /// Does not include layout, padding, borders, etc.
    pub(crate) fn text_sgr(&self) -> SgrStyle {
        let bold = self.get_bool(Props::BOLD);
        let italic = self.get_bool(Props::ITALIC);
        let underline = self.get_bool(Props::UNDERLINE);
        let strikethrough = self.get_bool(Props::STRIKETHROUGH);
        let reverse = self.get_bool(Props::REVERSE);
        let blink = self.get_bool(Props::BLINK);
        let faint = self.get_bool(Props::FAINT);
        let fg = if self.is_set(Props::FOREGROUND) {
            self.fg
        } else {
            Color::NoColor
        };
        let bg = if self.is_set(Props::BACKGROUND) {
            self.bg
        } else {
            Color::NoColor
        };
        let ul_style = if self.is_set(Props::UNDERLINE_STYLE) {
            self.underline_style
        } else {
            UnderlineStyle::None
        };
        let ul_color = if self.is_set(Props::UNDERLINE_COLOR) {
            self.underline_color
        } else {
            Color::NoColor
        };
        build_sgr(
            bold,
            italic,
            underline,
            strikethrough,
            reverse,
            blink,
            faint,
            fg,
            bg,
            ul_style,
            ul_color,
        )
    }

    /// Render one or more strings with this style applied.
    ///
    /// Multiple strings are joined with a space before rendering,
    /// matching lipgloss behavior.
    pub fn render(&self, strs: &[&str]) -> String {
        let s = strs.join(" ");

        if self.props == Props::empty() {
            return self.convert_tabs(&s);
        }

        let bold = self.get_bool(Props::BOLD);
        let italic = self.get_bool(Props::ITALIC);
        let underline = self.get_bool(Props::UNDERLINE);
        let strikethrough = self.get_bool(Props::STRIKETHROUGH);
        let reverse = self.get_bool(Props::REVERSE);
        let blink = self.get_bool(Props::BLINK);
        let faint = self.get_bool(Props::FAINT);

        let fg = if self.is_set(Props::FOREGROUND) {
            self.fg
        } else {
            Color::NoColor
        };
        let bg = if self.is_set(Props::BACKGROUND) {
            self.bg
        } else {
            Color::NoColor
        };
        let ul_style = if self.is_set(Props::UNDERLINE_STYLE) {
            self.underline_style
        } else {
            UnderlineStyle::None
        };
        let ul_color = if self.is_set(Props::UNDERLINE_COLOR) {
            self.underline_color
        } else {
            Color::NoColor
        };

        let width = if self.is_set(Props::WIDTH) {
            self.width as usize
        } else {
            0
        };
        let height = if self.is_set(Props::HEIGHT) {
            self.height as usize
        } else {
            0
        };
        let max_w = if self.is_set(Props::MAX_WIDTH) {
            self.max_width as usize
        } else {
            0
        };
        let max_h = if self.is_set(Props::MAX_HEIGHT) {
            self.max_height as usize
        } else {
            0
        };

        let h_align = if self.is_set(Props::ALIGN_H) {
            self.align_h
        } else {
            Position::LEFT
        };
        let v_align = if self.is_set(Props::ALIGN_V) {
            self.align_v
        } else {
            Position::TOP
        };

        let pt = self.padding_top as usize;
        let pr = self.padding_right as usize;
        let pb = self.padding_bottom as usize;
        let pl = self.padding_left as usize;

        let h_border = self.get_horizontal_border_size();
        let v_border = self.get_vertical_border_size();

        let color_whitespace = if self.is_set(Props::COLOR_WHITESPACE) {
            self.get_bool(Props::COLOR_WHITESPACE)
        } else {
            true
        };
        let inline = self.get_bool(Props::INLINE);

        // 1. Convert tabs
        let mut str = self.convert_tabs(&s);

        // 2. Normalize line endings
        str = str.replace("\r\n", "\n");

        // 3. Inline mode: strip newlines
        if inline {
            str = str.replace('\n', "");
        }

        // 4. Word wrap (if width > 0 and not inline)
        let effective_width = width.saturating_sub(h_border);
        if !inline && effective_width > 0 {
            let wrap_at = effective_width.saturating_sub(pl + pr);
            if wrap_at > 0 {
                str = wordwrap(&str, wrap_at);
            }
        }

        // 5. Apply text styling
        let te = build_sgr(
            bold,
            italic,
            underline,
            strikethrough,
            reverse,
            blink,
            faint,
            fg,
            bg,
            ul_style,
            ul_color,
        );
        let te_whitespace = build_whitespace_sgr(reverse, fg, bg, color_whitespace);

        {
            let mut out = String::new();
            for (i, line) in str.split('\n').enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                if te.is_empty() {
                    out.push_str(line);
                } else {
                    out.push_str(&te.styled(line));
                }
            }
            str = out;
        }

        // 6. Apply padding
        if !inline {
            if pl > 0 {
                let pad_str = " ".repeat(pl);
                let pad = if (color_whitespace || reverse) && !te_whitespace.is_empty() {
                    te_whitespace.styled(&pad_str)
                } else {
                    pad_str
                };
                str = pad_each_line_left(&str, &pad);
            }
            if pr > 0 {
                let pad_str = " ".repeat(pr);
                let pad = if (color_whitespace || reverse) && !te_whitespace.is_empty() {
                    te_whitespace.styled(&pad_str)
                } else {
                    pad_str
                };
                str = pad_each_line_right(&str, &pad);
            }
            if pt > 0 {
                str = "\n".repeat(pt) + &str;
            }
            if pb > 0 {
                str = str + &"\n".repeat(pb);
            }
        }

        // 7. Apply height + vertical alignment
        let effective_height = height.saturating_sub(v_border);
        if effective_height > 0 {
            str = align_text_vertical(&str, v_align, effective_height);
        }

        // 8. Apply horizontal alignment + width
        {
            let lines: Vec<&str> = str.split('\n').collect();
            let num_lines = lines.len();
            if num_lines > 1 || effective_width > 0 {
                let ws_style = if (color_whitespace || reverse) && !te_whitespace.is_empty() {
                    Some(&te_whitespace)
                } else {
                    None
                };
                str = align_text_horizontal(&str, h_align, effective_width, ws_style);
            }
        }

        // 9. Apply borders
        if !inline {
            str = self.apply_border(&str);
        }

        // 10. Apply margins
        if !inline {
            str = self.apply_margins(&str);
        }

        // 11. Apply max_width truncation
        if max_w > 0 {
            let lines: Vec<&str> = str.split('\n').collect();
            let mut out = String::new();
            for (i, line) in lines.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                out.push_str(&truncate(line, max_w, ""));
            }
            str = out;
        }

        // 12. Apply max_height truncation
        if max_h > 0 {
            let lines: Vec<&str> = str.split('\n').collect();
            let take = max_h.min(lines.len());
            str = lines[..take].join("\n");
        }

        // 13. Apply hyperlink (OSC 8)
        if let Some(ref url) = self.hyperlink {
            str = format!(
                "{}{}{}",
                ruse_ansi::hyperlink_open(url, ""),
                str,
                ruse_ansi::hyperlink_close(),
            );
        }

        // 14. Apply transform
        if let Some(ref f) = self.transform {
            str = f(&str);
        }

        str
    }

    /// Convenience for rendering a single string.
    pub fn render_one(&self, s: &str) -> String {
        self.render(&[s])
    }

    fn convert_tabs(&self, s: &str) -> String {
        let tw = if self.is_set(Props::TAB_WIDTH) {
            self.tab_width
        } else {
            4
        };
        match tw {
            w if w < 0 => s.to_string(),
            0 => s.replace('\t', ""),
            n => s.replace('\t', &" ".repeat(n as usize)),
        }
    }

    fn apply_border(&self, str: &str) -> String {
        let mut border = self.border_style.clone();

        let mut has_top = self.get_bool(Props::BORDER_TOP);
        let mut has_right = self.get_bool(Props::BORDER_RIGHT);
        let mut has_bottom = self.get_bool(Props::BORDER_BOTTOM);
        let mut has_left = self.get_bool(Props::BORDER_LEFT);

        // If a border style is set but no sides are explicitly configured,
        // render all sides.
        if self.is_border_style_set_without_sides() {
            has_top = true;
            has_right = true;
            has_bottom = true;
            has_left = true;
        }

        if border == NO_BORDER || (!has_top && !has_right && !has_bottom && !has_left) {
            return str.to_string();
        }

        let (lines, mut content_width) = get_lines(str);

        if has_left {
            let left = if border.left.is_empty() {
                " "
            } else {
                border.left
            };
            content_width += string_width(left);
            border.left = left;
        }
        if has_right {
            let right = if border.right.is_empty() {
                " "
            } else {
                border.right
            };
            content_width += string_width(right);
            border.right = right;
        }

        // Fix up corners
        if has_top && has_left && border.top_left.is_empty() {
            border.top_left = " ";
        }
        if has_top && has_right && border.top_right.is_empty() {
            border.top_right = " ";
        }
        if has_bottom && has_left && border.bottom_left.is_empty() {
            border.bottom_left = " ";
        }
        if has_bottom && has_right && border.bottom_right.is_empty() {
            border.bottom_right = " ";
        }

        // Clear corners for missing sides
        if has_top {
            if !has_left {
                border.top_left = "";
            }
            if !has_right {
                border.top_right = "";
            }
        }
        if has_bottom {
            if !has_left {
                border.bottom_left = "";
            }
            if !has_right {
                border.bottom_right = "";
            }
        }

        // Limit corners to first character
        border.top_left = first_char_str(border.top_left);
        border.top_right = first_char_str(border.top_right);
        border.bottom_left = first_char_str(border.bottom_left);
        border.bottom_right = first_char_str(border.bottom_right);

        let top_fg = if self.is_set(Props::BORDER_TOP_FG) {
            self.border_top_fg
        } else {
            Color::NoColor
        };
        let right_fg = if self.is_set(Props::BORDER_RIGHT_FG) {
            self.border_right_fg
        } else {
            Color::NoColor
        };
        let bottom_fg = if self.is_set(Props::BORDER_BOTTOM_FG) {
            self.border_bottom_fg
        } else {
            Color::NoColor
        };
        let left_fg = if self.is_set(Props::BORDER_LEFT_FG) {
            self.border_left_fg
        } else {
            Color::NoColor
        };

        let top_bg = if self.is_set(Props::BORDER_TOP_BG) {
            self.border_top_bg
        } else {
            Color::NoColor
        };
        let right_bg = if self.is_set(Props::BORDER_RIGHT_BG) {
            self.border_right_bg
        } else {
            Color::NoColor
        };
        let bottom_bg = if self.is_set(Props::BORDER_BOTTOM_BG) {
            self.border_bottom_bg
        } else {
            Color::NoColor
        };
        let left_bg = if self.is_set(Props::BORDER_LEFT_BG) {
            self.border_left_bg
        } else {
            Color::NoColor
        };

        let mut out = String::new();

        // Render top
        if has_top {
            let top_edge = render_horizontal_edge(
                border.top_left,
                border.top,
                border.top_right,
                content_width,
            );
            out.push_str(&style_border_str(&top_edge, top_fg, top_bg));
            out.push('\n');
        }

        // Render sides + content
        let left_chars: Vec<char> = border.left.chars().collect();
        let right_chars: Vec<char> = border.right.chars().collect();
        let mut li = 0usize;
        let mut ri = 0usize;

        for (i, line) in lines.iter().enumerate() {
            if has_left && !left_chars.is_empty() {
                let ch = left_chars[li % left_chars.len()];
                li += 1;
                out.push_str(&style_border_str(&ch.to_string(), left_fg, left_bg));
            }
            out.push_str(line);
            if has_right && !right_chars.is_empty() {
                let ch = right_chars[ri % right_chars.len()];
                ri += 1;
                out.push_str(&style_border_str(&ch.to_string(), right_fg, right_bg));
            }
            if i < lines.len() - 1 {
                out.push('\n');
            }
        }

        // Render bottom
        if has_bottom {
            let bottom_edge = render_horizontal_edge(
                border.bottom_left,
                border.bottom,
                border.bottom_right,
                content_width,
            );
            out.push('\n');
            out.push_str(&style_border_str(&bottom_edge, bottom_fg, bottom_bg));
        }

        out
    }

    fn apply_margins(&self, str: &str) -> String {
        let mt = self.margin_top as usize;
        let mr = self.margin_right as usize;
        let mb = self.margin_bottom as usize;
        let ml = self.margin_left as usize;

        if mt == 0 && mr == 0 && mb == 0 && ml == 0 {
            return str.to_string();
        }

        let margin_bg = if self.is_set(Props::MARGIN_BG) {
            self.margin_bg
        } else {
            Color::NoColor
        };
        let style = build_bg_sgr(margin_bg);

        let mut result = str.to_string();

        // Left/right margin
        if ml > 0 {
            let sp = " ".repeat(ml);
            let pad = if style.is_empty() {
                sp
            } else {
                style.styled(&sp)
            };
            result = pad_each_line_left(&result, &pad);
        }
        if mr > 0 {
            let sp = " ".repeat(mr);
            let pad = if style.is_empty() {
                sp
            } else {
                style.styled(&sp)
            };
            result = pad_each_line_right(&result, &pad);
        }

        // Top/bottom margin
        if mt > 0 || mb > 0 {
            let (_, w) = get_lines(&result);
            let spaces = " ".repeat(w);
            let empty_line = if style.is_empty() {
                spaces.clone()
            } else {
                style.styled(&spaces)
            };

            if mt > 0 {
                let top = (0..mt)
                    .map(|_| format!("{empty_line}\n"))
                    .collect::<String>();
                result = top + &result;
            }
            if mb > 0 {
                let bottom = (0..mb)
                    .map(|_| format!("\n{empty_line}"))
                    .collect::<String>();
                result = result + &bottom;
            }
        }

        result
    }
}

/// Build the main text SGR style.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_sgr(
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    reverse: bool,
    blink: bool,
    faint: bool,
    fg: Color,
    bg: Color,
    ul_style: UnderlineStyle,
    ul_color: Color,
) -> SgrStyle {
    let mut te = SgrStyle::new();
    if bold {
        te = te.bold();
    }
    if italic {
        te = te.italic();
    }
    // Apply underline: if an explicit underline style is set, use it;
    // otherwise fall back to the simple underline boolean.
    match ul_style {
        UnderlineStyle::None => {
            if underline {
                te = te.underline();
            }
        }
        UnderlineStyle::Single => te = te.underline(),
        UnderlineStyle::Double => te = te.double_underline(),
        UnderlineStyle::Curly => te = te.curly_underline(),
        UnderlineStyle::Dotted => te = te.dotted_underline(),
        UnderlineStyle::Dashed => te = te.dashed_underline(),
    }
    if strikethrough {
        te = te.strikethrough();
    }
    if reverse {
        te = te.reverse();
    }
    if blink {
        te = te.blink();
    }
    if faint {
        te = te.faint();
    }
    te = apply_fg(te, fg);
    te = apply_bg(te, bg);
    te = apply_ul(te, ul_color);
    te
}

/// Build a whitespace SGR (for padding/alignment areas).
fn build_whitespace_sgr(reverse: bool, fg: Color, bg: Color, color_whitespace: bool) -> SgrStyle {
    let mut te = SgrStyle::new();
    if reverse {
        te = te.reverse();
        te = apply_fg(te, fg);
    }
    if color_whitespace {
        te = apply_bg(te, bg);
    }
    te
}

fn build_bg_sgr(bg: Color) -> SgrStyle {
    let mut te = SgrStyle::new();
    te = apply_bg(te, bg);
    te
}

fn apply_fg(te: SgrStyle, c: Color) -> SgrStyle {
    match c {
        Color::NoColor => te,
        Color::Basic(n) => te.fg_basic(n),
        Color::Indexed(n) => te.fg_256(n),
        Color::Rgb { r, g, b } => te.fg_rgb(r, g, b),
    }
}

fn apply_bg(te: SgrStyle, c: Color) -> SgrStyle {
    match c {
        Color::NoColor => te,
        Color::Basic(n) => te.bg_basic(n),
        Color::Indexed(n) => te.bg_256(n),
        Color::Rgb { r, g, b } => te.bg_rgb(r, g, b),
    }
}

fn apply_ul(te: SgrStyle, c: Color) -> SgrStyle {
    match c {
        Color::NoColor => te,
        // Basic colors 0-15 map to indexed 0-15
        Color::Basic(n) => te.ul_256(n),
        Color::Indexed(n) => te.ul_256(n),
        Color::Rgb { r, g, b } => te.ul_rgb(r, g, b),
    }
}

fn style_border_str(s: &str, fg: Color, bg: Color) -> String {
    if fg == Color::NoColor && bg == Color::NoColor {
        return s.to_string();
    }
    let mut te = SgrStyle::new();
    te = apply_fg(te, fg);
    te = apply_bg(te, bg);
    te.styled(s)
}

/// Render a horizontal border edge (top or bottom).
fn render_horizontal_edge(left: &str, middle: &str, right: &str, width: usize) -> String {
    let mid = if middle.is_empty() { " " } else { middle };
    let left_w = string_width(left);
    let right_w = string_width(right);
    let fill_w = width.saturating_sub(left_w + right_w);

    let mid_chars: Vec<char> = mid.chars().collect();
    let mut out = String::from(left);
    let mut col = 0;
    let mut j = 0;
    while col < fill_w {
        let ch = mid_chars[j % mid_chars.len()];
        out.push(ch);
        col += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        j += 1;
    }
    out.push_str(right);
    out
}

/// Split a string into lines and return the widest line width.
pub(crate) fn get_lines(s: &str) -> (Vec<&str>, usize) {
    let lines: Vec<&str> = s.split('\n').collect();
    let widest = lines.iter().map(|l| string_width(l)).max().unwrap_or(0);
    (lines, widest)
}

fn pad_each_line_left(s: &str, pad: &str) -> String {
    let mut out = String::new();
    for (i, line) in s.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(pad);
        out.push_str(line);
    }
    out
}

fn pad_each_line_right(s: &str, pad: &str) -> String {
    let mut out = String::new();
    for (i, line) in s.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(line);
        out.push_str(pad);
    }
    out
}

fn align_text_horizontal(
    str: &str,
    pos: Position,
    width: usize,
    style: Option<&SgrStyle>,
) -> String {
    let (lines, widest) = get_lines(str);
    let mut out = String::new();

    for (i, line) in lines.iter().enumerate() {
        let line_width = string_width(line);
        let short_amount = (widest - line_width) + width.saturating_sub(widest.max(line_width));

        let mut l = line.to_string();

        if short_amount > 0 {
            if pos == Position::RIGHT {
                let sp = " ".repeat(short_amount);
                let sp = if let Some(st) = style {
                    st.styled(&sp)
                } else {
                    sp
                };
                l = sp + &l;
            } else if pos == Position::CENTER {
                let left = short_amount / 2;
                let right = left + short_amount % 2;
                let left_sp = " ".repeat(left);
                let right_sp = " ".repeat(right);
                let left_sp = if let Some(st) = style {
                    st.styled(&left_sp)
                } else {
                    left_sp
                };
                let right_sp = if let Some(st) = style {
                    st.styled(&right_sp)
                } else {
                    right_sp
                };
                l = left_sp + &l + &right_sp;
            } else {
                // Left alignment (default)
                let sp = " ".repeat(short_amount);
                let sp = if let Some(st) = style {
                    st.styled(&sp)
                } else {
                    sp
                };
                l = l + &sp;
            }
        }

        if i > 0 {
            out.push('\n');
        }
        out.push_str(&l);
    }

    out
}

fn align_text_vertical(str: &str, pos: Position, height: usize) -> String {
    let str_height = str.chars().filter(|&c| c == '\n').count() + 1;
    if height <= str_height {
        return str.to_string();
    }

    let gap = height - str_height;

    if pos == Position::TOP {
        format!("{}{}", str, "\n".repeat(gap))
    } else if pos == Position::BOTTOM {
        format!("{}{}", "\n".repeat(gap), str)
    } else {
        // Center
        let top = gap / 2;
        let bottom = gap - top;
        format!("{}{}{}", "\n".repeat(top), str, "\n".repeat(bottom))
    }
}

/// Return the first character of a static str as a static str,
/// or the whole str if empty.
fn first_char_str(s: &'static str) -> &'static str {
    if s.is_empty() {
        return s;
    }
    let end = s.char_indices().nth(1).map_or(s.len(), |(i, _)| i);
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::border::NORMAL_BORDER;

    #[test]
    fn test_render_plain() {
        let s = Style::new();
        assert_eq!(s.render(&["hello"]), "hello");
    }

    #[test]
    fn test_render_bold() {
        let s = Style::new().bold(true);
        let result = s.render(&["hello"]);
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_render_multiple_strings() {
        let s = Style::new();
        assert_eq!(s.render(&["hello", "world"]), "hello world");
    }

    #[test]
    fn test_render_padding() {
        let s = Style::new().padding(&[0, 2]);
        let result = s.render(&["hi"]);
        // Should have 2 spaces on each side
        assert_eq!(result, "  hi  ");
    }

    #[test]
    fn test_render_width() {
        let s = Style::new().width(10);
        let result = s.render(&["hi"]);
        assert_eq!(string_width(&result), 10);
    }

    #[test]
    fn test_render_align_right() {
        let s = Style::new().width(10).align_horizontal(Position::RIGHT);
        let result = s.render(&["hi"]);
        assert_eq!(result, "        hi");
    }

    #[test]
    fn test_render_align_center() {
        let s = Style::new().width(10).align_horizontal(Position::CENTER);
        let result = s.render(&["hi"]);
        assert_eq!(result, "    hi    ");
    }

    #[test]
    fn test_render_border() {
        let s = Style::new().border(NORMAL_BORDER, &[true]);
        let result = s.render(&["hi"]);
        assert!(result.contains("┌"));
        assert!(result.contains("└"));
        assert!(result.contains("│"));
    }

    #[test]
    fn test_render_margin() {
        let s = Style::new().margin_left(2);
        let result = s.render(&["hi"]);
        assert_eq!(result, "  hi");
    }

    #[test]
    fn test_render_max_width() {
        let s = Style::new().max_width(5);
        let result = s.render(&["hello world"]);
        assert!(string_width(&result) <= 5);
    }

    #[test]
    fn test_render_max_height() {
        let s = Style::new().max_height(2);
        let result = s.render(&["line1\nline2\nline3"]);
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_render_inline() {
        let s = Style::new().inline(true);
        let result = s.render(&["line1\nline2"]);
        assert!(!result.contains('\n'));
    }

    #[test]
    fn test_render_tab_conversion() {
        let s = Style::new().tab_width(2);
        let result = s.render(&["a\tb"]);
        assert_eq!(result, "a  b");
    }

    #[test]
    fn test_render_tab_disabled() {
        let s = Style::new().tab_width(-1);
        let result = s.render(&["a\tb"]);
        assert_eq!(result, "a\tb");
    }

    #[test]
    fn test_get_lines() {
        let (lines, w) = get_lines("hello\nworld!");
        assert_eq!(lines, vec!["hello", "world!"]);
        assert_eq!(w, 6);
    }

    #[test]
    fn test_first_char_str() {
        assert_eq!(first_char_str(""), "");
        assert_eq!(first_char_str("a"), "a");
        assert_eq!(first_char_str("ab"), "a");
        // Multi-byte
        assert_eq!(first_char_str("\u{250C}abc"), "\u{250C}");
    }

    #[test]
    fn test_render_underline_single() {
        let s = Style::new().set_underline_style(crate::style::UnderlineStyle::Single);
        let result = s.render(&["hi"]);
        // SGR 4 = single underline
        assert!(result.contains("\x1b[4m"));
    }

    #[test]
    fn test_render_underline_curly() {
        let s = Style::new().set_underline_style(crate::style::UnderlineStyle::Curly);
        let result = s.render(&["hi"]);
        // SGR 4:3 = curly underline
        assert!(result.contains("4:3"));
    }

    #[test]
    fn test_render_underline_double() {
        let s = Style::new().set_underline_style(crate::style::UnderlineStyle::Double);
        let result = s.render(&["hi"]);
        // SGR 21 = double underline
        assert!(result.contains("\x1b[21m") || result.contains(";21m") || result.contains(";21;"));
    }

    #[test]
    fn test_render_underline_color() {
        use ruse_colorprofile::Color;
        let s = Style::new()
            .set_underline_style(crate::style::UnderlineStyle::Curly)
            .underline_color(Color::Rgb { r: 255, g: 0, b: 0 });
        let result = s.render(&["hi"]);
        // Should contain curly underline AND underline color
        assert!(result.contains("4:3"));
        assert!(result.contains("58;2;255;0;0"));
    }

    #[test]
    fn test_render_underline_dotted() {
        let s = Style::new().set_underline_style(crate::style::UnderlineStyle::Dotted);
        let result = s.render(&["hi"]);
        assert!(result.contains("4:4"));
    }

    #[test]
    fn test_render_underline_dashed() {
        let s = Style::new().set_underline_style(crate::style::UnderlineStyle::Dashed);
        let result = s.render(&["hi"]);
        assert!(result.contains("4:5"));
    }

    #[test]
    fn test_render_transform() {
        let s = Style::new().transform(|s| s.to_uppercase());
        let result = s.render(&["hello"]);
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_render_transform_with_style() {
        let s = Style::new().bold(true).transform(|s| s.to_uppercase());
        let result = s.render(&["hello"]);
        // Transform applies to the entire rendered string including ANSI codes
        let upper = result.to_uppercase();
        assert_eq!(result, upper);
    }

    #[test]
    fn test_render_hyperlink() {
        let s = Style::new().hyperlink("https://example.com");
        let result = s.render(&["click"]);
        assert!(result.contains("\x1b]8;"));
        assert!(result.contains("https://example.com"));
        assert!(result.contains("click"));
    }
}
