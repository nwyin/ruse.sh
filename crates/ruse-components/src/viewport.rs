use ruse_runtime::{Cmd, MouseButton, Msg};
use ruse_style::Style;

use crate::key::Binding;

/// Context passed to the gutter rendering function.
pub struct GutterContext {
    /// Zero-based line index in the content.
    pub index: usize,
    /// Total number of lines in the content.
    pub total_lines: usize,
    /// Whether this is a soft-wrapped line (not the first line of a logical line).
    pub soft: bool,
}

/// Key bindings for the Viewport component.
pub struct ViewportKeyMap {
    pub up: Binding,
    pub down: Binding,
    pub page_up: Binding,
    pub page_down: Binding,
    pub half_page_up: Binding,
    pub half_page_down: Binding,
    pub goto_top: Binding,
    pub goto_bottom: Binding,
}

impl Default for ViewportKeyMap {
    fn default() -> Self {
        Self {
            up: Binding::new(&["up", "k"], "↑/k", "up"),
            down: Binding::new(&["down", "j"], "↓/j", "down"),
            page_up: Binding::new(&["b", "pgup"], "b/pgup", "page up"),
            page_down: Binding::new(&["f", "pgdn", "space"], "f/pgdn", "page down"),
            half_page_up: Binding::new(&["u", "ctrl+u"], "u", "½ page up"),
            half_page_down: Binding::new(&["d", "ctrl+d"], "d", "½ page down"),
            goto_top: Binding::new(&["home", "g"], "g/home", "go to start"),
            goto_bottom: Binding::new(&["end", "G"], "G/end", "go to end"),
        }
    }
}

pub struct Viewport {
    pub key_map: ViewportKeyMap,
    width: usize,
    height: usize,
    y_offset: usize,
    x_offset: usize,
    horizontal_step: usize,
    lines: Vec<String>,
    pub mouse_wheel_enabled: bool,
    pub mouse_wheel_delta: usize,
    pub style: Style,
    pub gutter_func: Option<Box<dyn Fn(GutterContext) -> String + Send>>,
    pub style_line_func: Option<Box<dyn Fn(usize) -> Style + Send>>,
}

impl Viewport {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            key_map: ViewportKeyMap::default(),
            width,
            height,
            y_offset: 0,
            x_offset: 0,
            horizontal_step: 4,
            lines: Vec::new(),
            mouse_wheel_enabled: true,
            mouse_wheel_delta: 3,
            style: Style::new(),
            gutter_func: None,
            style_line_func: None,
        }
    }

    pub fn set_content(&mut self, content: &str) {
        self.lines = content.lines().map(|l| l.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        // Clamp y_offset
        self.clamp_y_offset();
    }

    pub fn set_width(&mut self, w: usize) {
        self.width = w;
    }

    pub fn set_height(&mut self, h: usize) {
        self.height = h;
        self.clamp_y_offset();
    }

    pub fn y_offset(&self) -> usize {
        self.y_offset
    }

    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    pub fn at_top(&self) -> bool {
        self.y_offset == 0
    }

    pub fn at_bottom(&self) -> bool {
        if self.lines.len() <= self.height {
            return true;
        }
        self.y_offset >= self.lines.len() - self.height
    }

    pub fn scroll_percent(&self) -> f64 {
        if self.lines.len() <= self.height {
            return 1.0;
        }
        let max_offset = self.lines.len() - self.height;
        if max_offset == 0 {
            return 1.0;
        }
        self.y_offset as f64 / max_offset as f64
    }

    pub fn line_down(&mut self, n: usize) {
        self.y_offset = self.y_offset.saturating_add(n);
        self.clamp_y_offset();
    }

    pub fn line_up(&mut self, n: usize) {
        self.y_offset = self.y_offset.saturating_sub(n);
    }

    pub fn page_down(&mut self) {
        self.line_down(self.height);
    }

    pub fn page_up(&mut self) {
        self.line_up(self.height);
    }

    pub fn half_page_down(&mut self) {
        self.line_down(self.height / 2);
    }

    pub fn half_page_up(&mut self) {
        self.line_up(self.height / 2);
    }

    pub fn line_left(&mut self) {
        self.x_offset = self.x_offset.saturating_sub(self.horizontal_step);
    }

    pub fn line_right(&mut self) {
        self.x_offset = self.x_offset.saturating_add(self.horizontal_step);
    }

    pub fn x_offset(&self) -> usize {
        self.x_offset
    }

    pub fn set_horizontal_step(&mut self, step: usize) {
        self.horizontal_step = step;
    }

    pub fn goto_top(&mut self) {
        self.y_offset = 0;
    }

    pub fn goto_bottom(&mut self) {
        if self.lines.len() > self.height {
            self.y_offset = self.lines.len() - self.height;
        }
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        match msg {
            Msg::KeyPress(key) => {
                if self.key_map.up.matches(key) {
                    self.line_up(1);
                } else if self.key_map.down.matches(key) {
                    self.line_down(1);
                } else if self.key_map.page_up.matches(key) {
                    self.page_up();
                } else if self.key_map.page_down.matches(key) {
                    self.page_down();
                } else if self.key_map.half_page_up.matches(key) {
                    self.half_page_up();
                } else if self.key_map.half_page_down.matches(key) {
                    self.half_page_down();
                } else if self.key_map.goto_top.matches(key) {
                    self.goto_top();
                } else if self.key_map.goto_bottom.matches(key) {
                    self.goto_bottom();
                }
            }
            Msg::MouseWheel(mouse) if self.mouse_wheel_enabled => match mouse.button {
                MouseButton::WheelUp => self.line_up(self.mouse_wheel_delta),
                MouseButton::WheelDown => self.line_down(self.mouse_wheel_delta),
                _ => {}
            },
            _ => {}
        }
        None
    }

    pub fn view(&self) -> String {
        if self.lines.is_empty() || self.height == 0 {
            return String::new();
        }

        let end = (self.y_offset + self.height).min(self.lines.len());
        let visible = &self.lines[self.y_offset..end];
        let total_lines = self.lines.len();

        let mut out = String::new();
        for (i, line) in visible.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }

            // Gutter
            let gutter_str = if let Some(ref gutter_fn) = self.gutter_func {
                let ctx = GutterContext {
                    index: self.y_offset + i,
                    total_lines,
                    soft: false,
                };
                gutter_fn(ctx)
            } else {
                String::new()
            };
            let gutter_width = ruse_ansi::string_width(&gutter_str);

            // Apply horizontal offset (x_offset) to content
            let shifted_line = if self.x_offset > 0 {
                let chars: Vec<char> = line.chars().collect();
                if self.x_offset < chars.len() {
                    chars[self.x_offset..].iter().collect()
                } else {
                    String::new()
                }
            } else {
                line.clone()
            };

            // Apply per-line style if provided
            let styled_line = if let Some(ref style_fn) = self.style_line_func {
                let line_style = style_fn(self.y_offset + i);
                line_style.render(&[&shifted_line])
            } else {
                shifted_line
            };

            // Truncate/pad line to width if width is set
            let content_width = self.width.saturating_sub(gutter_width);
            let displayed = if content_width > 0 {
                let w = ruse_ansi::string_width(&styled_line);
                if w > content_width {
                    ruse_ansi::truncate(&styled_line, content_width, "")
                } else {
                    let padding = content_width - w;
                    format!("{}{}", styled_line, " ".repeat(padding))
                }
            } else {
                styled_line
            };

            out.push_str(&gutter_str);
            out.push_str(&displayed);
        }

        // Pad with empty lines if we don't have enough content
        let visible_count = end - self.y_offset;
        for _ in visible_count..self.height {
            out.push('\n');
            if self.width > 0 {
                out.push_str(&" ".repeat(self.width));
            }
        }

        out
    }

    fn clamp_y_offset(&mut self) {
        if self.lines.len() <= self.height {
            self.y_offset = 0;
        } else if self.y_offset > self.lines.len() - self.height {
            self.y_offset = self.lines.len() - self.height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_viewport() {
        let mut vp = Viewport::new(20, 3);
        vp.set_content("line1\nline2\nline3\nline4\nline5");
        assert_eq!(vp.total_lines(), 5);
        assert_eq!(vp.y_offset(), 0);
        assert!(vp.at_top());
        assert!(!vp.at_bottom());
    }

    #[test]
    fn test_scroll_down() {
        let mut vp = Viewport::new(20, 3);
        vp.set_content("line1\nline2\nline3\nline4\nline5");
        vp.line_down(1);
        assert_eq!(vp.y_offset(), 1);
        assert!(!vp.at_top());
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut vp = Viewport::new(20, 3);
        vp.set_content("line1\nline2\nline3\nline4\nline5");
        vp.goto_bottom();
        assert_eq!(vp.y_offset(), 2);
        assert!(vp.at_bottom());
    }

    #[test]
    fn test_scroll_clamp() {
        let mut vp = Viewport::new(20, 3);
        vp.set_content("line1\nline2\nline3\nline4\nline5");
        vp.line_down(100);
        assert_eq!(vp.y_offset(), 2); // clamped to max
    }

    #[test]
    fn test_content_fits() {
        let mut vp = Viewport::new(20, 5);
        vp.set_content("line1\nline2\nline3");
        assert!(vp.at_top());
        assert!(vp.at_bottom());
        assert_eq!(vp.scroll_percent(), 1.0);
    }

    #[test]
    fn test_scroll_percent() {
        let mut vp = Viewport::new(20, 3);
        vp.set_content("line1\nline2\nline3\nline4\nline5");
        assert_eq!(vp.scroll_percent(), 0.0);
        vp.goto_bottom();
        assert_eq!(vp.scroll_percent(), 1.0);
        vp.line_up(1);
        assert_eq!(vp.scroll_percent(), 0.5);
    }

    #[test]
    fn test_page_down_up() {
        let mut vp = Viewport::new(20, 2);
        vp.set_content("a\nb\nc\nd\ne\nf");
        vp.page_down();
        assert_eq!(vp.y_offset(), 2);
        vp.page_up();
        assert_eq!(vp.y_offset(), 0);
    }

    #[test]
    fn test_half_page() {
        let mut vp = Viewport::new(20, 4);
        vp.set_content("a\nb\nc\nd\ne\nf\ng\nh");
        vp.half_page_down();
        assert_eq!(vp.y_offset(), 2);
        vp.half_page_up();
        assert_eq!(vp.y_offset(), 0);
    }

    #[test]
    fn test_view_output() {
        let mut vp = Viewport::new(0, 3);
        vp.set_content("line1\nline2\nline3\nline4");
        let view = vp.view();
        assert!(view.contains("line1"));
        assert!(view.contains("line2"));
        assert!(view.contains("line3"));
        assert!(!view.contains("line4"));
    }

    #[test]
    fn test_view_scrolled() {
        let mut vp = Viewport::new(0, 2);
        vp.set_content("line1\nline2\nline3\nline4");
        vp.line_down(2);
        let view = vp.view();
        assert!(!view.contains("line1"));
        assert!(!view.contains("line2"));
        assert!(view.contains("line3"));
        assert!(view.contains("line4"));
    }
}
