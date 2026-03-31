use ruse_ansi::Rect;

/// The view returned by `Model::view()`, describing what to render.
#[derive(Default)]
pub struct View {
    pub content: String,
    /// Region-based rendering: each entry draws a styled string into a bounded rectangle.
    /// When non-empty, `content` is ignored and regions are drawn independently.
    pub regions: Vec<(Rect, String)>,
    pub alt_screen: bool,
    pub mouse_mode: MouseMode,
    pub report_focus: bool,
    pub cursor: Option<CursorView>,
}

impl View {
    /// Create a new view with the given content string.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Default::default()
        }
    }

    /// Add a region: draw `content` into the given rectangle.
    pub fn with_region(mut self, rect: Rect, content: impl Into<String>) -> Self {
        self.regions.push((rect, content.into()));
        self
    }

    /// Set all regions at once.
    pub fn with_regions(mut self, regions: Vec<(Rect, String)>) -> Self {
        self.regions = regions;
        self
    }

    /// Enable alternate screen mode for this view.
    pub fn with_alt_screen(mut self) -> Self {
        self.alt_screen = true;
        self
    }

    /// Set the mouse mode for this view.
    pub fn with_mouse(mut self, mode: MouseMode) -> Self {
        self.mouse_mode = mode;
        self
    }

    /// Enable focus reporting for this view.
    pub fn with_focus_report(mut self) -> Self {
        self.report_focus = true;
        self
    }

    /// Set the cursor for this view.
    pub fn with_cursor(mut self, cursor: CursorView) -> Self {
        self.cursor = Some(cursor);
        self
    }
}

/// Mouse tracking mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseMode {
    /// No mouse events.
    #[default]
    None,
    /// Mouse click, release, wheel, and drag events.
    CellMotion,
    /// All mouse events including movement without buttons pressed.
    AllMotion,
}

/// Cursor position and appearance for the view.
#[derive(Debug, Clone)]
pub struct CursorView {
    pub x: u16,
    pub y: u16,
    pub shape: CursorShape,
    pub visible: bool,
}

/// Cursor shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorShape {
    #[default]
    Block,
    Underline,
    Bar,
}
