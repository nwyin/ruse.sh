use rouge_runtime::{Cmd, KeyCode, Modifiers, Msg, MouseButton};
use rouge_style::Style;

pub struct Viewport {
    width: usize,
    height: usize,
    y_offset: usize,
    lines: Vec<String>,
    pub mouse_wheel_enabled: bool,
    pub mouse_wheel_delta: usize,
    pub style: Style,
}

impl Viewport {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            y_offset: 0,
            lines: Vec::new(),
            mouse_wheel_enabled: true,
            mouse_wheel_delta: 3,
            style: Style::new(),
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
                let ctrl = key.modifiers.contains(Modifiers::CTRL);
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => self.line_up(1),
                    KeyCode::Down | KeyCode::Char('j') => self.line_down(1),
                    KeyCode::PageUp => self.page_up(),
                    KeyCode::PageDown => self.page_down(),
                    KeyCode::Home | KeyCode::Char('g') => self.goto_top(),
                    KeyCode::End | KeyCode::Char('G') => self.goto_bottom(),
                    KeyCode::Char('u') if ctrl => self.half_page_up(),
                    KeyCode::Char('d') if ctrl => self.half_page_down(),
                    _ => {}
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

        let mut out = String::new();
        for (i, line) in visible.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            // Truncate/pad line to width if width is set
            let displayed = if self.width > 0 {
                let w = rouge_ansi::string_width(line);
                if w > self.width {
                    rouge_ansi::truncate(line, self.width, "")
                } else {
                    let padding = self.width - w;
                    format!("{}{}", line, " ".repeat(padding))
                }
            } else {
                line.clone()
            };
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
