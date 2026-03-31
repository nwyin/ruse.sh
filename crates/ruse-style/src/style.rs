use std::sync::Arc;

use bitflags::bitflags;
use ruse_colorprofile::Color;

use crate::border::{Border, NO_BORDER};
use crate::position::Position;

/// Underline style variants for terminal rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnderlineStyle {
    #[default]
    None,
    /// Standard single underline (SGR 4).
    Single,
    /// Double underline (SGR 21).
    Double,
    /// Curly/wavy underline (SGR 4:3).
    Curly,
    /// Dotted underline (SGR 4:4).
    Dotted,
    /// Dashed underline (SGR 4:5).
    Dashed,
}

bitflags! {
    /// Tracks which properties have been explicitly set on a [`Style`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Props: u64 {
        const BOLD              = 1 << 0;
        const ITALIC            = 1 << 1;
        const UNDERLINE         = 1 << 2;
        const STRIKETHROUGH     = 1 << 3;
        const REVERSE           = 1 << 4;
        const BLINK             = 1 << 5;
        const FAINT             = 1 << 6;
        const UNDERLINE_SPACES  = 1 << 7;
        const STRIKETHROUGH_SPACES = 1 << 8;
        const COLOR_WHITESPACE  = 1 << 9;
        const FOREGROUND        = 1 << 10;
        const BACKGROUND        = 1 << 11;
        const WIDTH             = 1 << 12;
        const HEIGHT            = 1 << 13;
        const ALIGN_H           = 1 << 14;
        const ALIGN_V           = 1 << 15;
        const PADDING_TOP       = 1 << 16;
        const PADDING_RIGHT     = 1 << 17;
        const PADDING_BOTTOM    = 1 << 18;
        const PADDING_LEFT      = 1 << 19;
        const MARGIN_TOP        = 1 << 20;
        const MARGIN_RIGHT      = 1 << 21;
        const MARGIN_BOTTOM     = 1 << 22;
        const MARGIN_LEFT       = 1 << 23;
        const MARGIN_BG         = 1 << 24;
        const BORDER_STYLE      = 1 << 25;
        const BORDER_TOP        = 1 << 26;
        const BORDER_RIGHT      = 1 << 27;
        const BORDER_BOTTOM     = 1 << 28;
        const BORDER_LEFT       = 1 << 29;
        const BORDER_TOP_FG     = 1 << 30;
        const BORDER_RIGHT_FG   = 1 << 31;
        const BORDER_BOTTOM_FG  = 1 << 32;
        const BORDER_LEFT_FG    = 1 << 33;
        const BORDER_TOP_BG     = 1 << 34;
        const BORDER_RIGHT_BG   = 1 << 35;
        const BORDER_BOTTOM_BG  = 1 << 36;
        const BORDER_LEFT_BG    = 1 << 37;
        const MAX_WIDTH         = 1 << 38;
        const MAX_HEIGHT        = 1 << 39;
        const TAB_WIDTH         = 1 << 40;
        const INLINE            = 1 << 41;
        const UNDERLINE_STYLE   = 1 << 42;
        const UNDERLINE_COLOR   = 1 << 43;
        const TRANSFORM         = 1 << 44;
        const HYPERLINK         = 1 << 45;
    }
}

/// The set of boolean-valued attribute flags.
const BOOL_ATTRS: Props = Props::BOLD
    .union(Props::ITALIC)
    .union(Props::UNDERLINE)
    .union(Props::STRIKETHROUGH)
    .union(Props::REVERSE)
    .union(Props::BLINK)
    .union(Props::FAINT)
    .union(Props::UNDERLINE_SPACES)
    .union(Props::STRIKETHROUGH_SPACES)
    .union(Props::COLOR_WHITESPACE)
    .union(Props::INLINE);

/// A composable terminal style. All builder methods consume and return `Self`.
#[derive(Clone)]
pub struct Style {
    /// Which properties have been explicitly set.
    pub(crate) props: Props,
    /// Boolean attribute values (only meaningful for bits also set in `props`).
    pub(crate) attrs: Props,

    // Underline
    pub(crate) underline_style: UnderlineStyle,
    pub(crate) underline_color: Color,

    // Colors
    pub(crate) fg: Color,
    pub(crate) bg: Color,
    pub(crate) margin_bg: Color,
    pub(crate) border_top_fg: Color,
    pub(crate) border_right_fg: Color,
    pub(crate) border_bottom_fg: Color,
    pub(crate) border_left_fg: Color,
    pub(crate) border_top_bg: Color,
    pub(crate) border_right_bg: Color,
    pub(crate) border_bottom_bg: Color,
    pub(crate) border_left_bg: Color,

    // Dimensions
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) max_width: u16,
    pub(crate) max_height: u16,

    // Padding
    pub(crate) padding_top: u16,
    pub(crate) padding_right: u16,
    pub(crate) padding_bottom: u16,
    pub(crate) padding_left: u16,

    // Margin
    pub(crate) margin_top: u16,
    pub(crate) margin_right: u16,
    pub(crate) margin_bottom: u16,
    pub(crate) margin_left: u16,

    // Alignment
    pub(crate) align_h: Position,
    pub(crate) align_v: Position,

    // Border
    pub(crate) border_style: Border,

    // Tab width
    pub(crate) tab_width: i8,

    // Transform hook
    pub(crate) transform: Option<Arc<dyn Fn(&str) -> String + Send + Sync>>,

    // Hyperlink
    pub(crate) hyperlink: Option<String>,
}

impl std::fmt::Debug for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Style")
            .field("props", &self.props)
            .field("attrs", &self.attrs)
            .field("fg", &self.fg)
            .field("bg", &self.bg)
            .field("transform", &self.transform.as_ref().map(|_| ".."))
            .field("hyperlink", &self.hyperlink)
            .finish_non_exhaustive()
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

impl Style {
    /// Create a new empty style with no properties set.
    pub fn new() -> Self {
        Self {
            props: Props::empty(),
            attrs: Props::empty(),
            underline_style: UnderlineStyle::None,
            underline_color: Color::NoColor,
            fg: Color::NoColor,
            bg: Color::NoColor,
            margin_bg: Color::NoColor,
            border_top_fg: Color::NoColor,
            border_right_fg: Color::NoColor,
            border_bottom_fg: Color::NoColor,
            border_left_fg: Color::NoColor,
            border_top_bg: Color::NoColor,
            border_right_bg: Color::NoColor,
            border_bottom_bg: Color::NoColor,
            border_left_bg: Color::NoColor,
            width: 0,
            height: 0,
            max_width: 0,
            max_height: 0,
            padding_top: 0,
            padding_right: 0,
            padding_bottom: 0,
            padding_left: 0,
            margin_top: 0,
            margin_right: 0,
            margin_bottom: 0,
            margin_left: 0,
            align_h: Position::LEFT,
            align_v: Position::TOP,
            border_style: NO_BORDER,
            tab_width: 4,
            transform: None,
            hyperlink: None,
        }
    }

    // --- internal helpers ---

    fn set_bool(mut self, prop: Props, v: bool) -> Self {
        self.props |= prop;
        if v {
            self.attrs |= prop;
        } else {
            self.attrs -= prop;
        }
        self
    }

    pub(crate) fn get_bool(&self, prop: Props) -> bool {
        self.props.contains(prop) && self.attrs.contains(prop)
    }

    pub(crate) fn is_set(&self, prop: Props) -> bool {
        self.props.contains(prop)
    }

    // --- text attribute builders ---

    pub fn bold(self, v: bool) -> Self {
        self.set_bool(Props::BOLD, v)
    }

    pub fn italic(self, v: bool) -> Self {
        self.set_bool(Props::ITALIC, v)
    }

    pub fn underline(self, v: bool) -> Self {
        self.set_bool(Props::UNDERLINE, v)
    }

    pub fn strikethrough(self, v: bool) -> Self {
        self.set_bool(Props::STRIKETHROUGH, v)
    }

    pub fn reverse(self, v: bool) -> Self {
        self.set_bool(Props::REVERSE, v)
    }

    pub fn blink(self, v: bool) -> Self {
        self.set_bool(Props::BLINK, v)
    }

    pub fn faint(self, v: bool) -> Self {
        self.set_bool(Props::FAINT, v)
    }

    pub fn underline_spaces(self, v: bool) -> Self {
        self.set_bool(Props::UNDERLINE_SPACES, v)
    }

    /// Set the underline style. This also implicitly enables the `UNDERLINE`
    /// boolean attribute when the style is not `None`.
    pub fn set_underline_style(mut self, style: UnderlineStyle) -> Self {
        self.props |= Props::UNDERLINE_STYLE;
        self.underline_style = style;
        // Automatically set the underline bool when a non-None style is specified
        if style != UnderlineStyle::None {
            self = self.underline(true);
        }
        self
    }

    /// Set the underline color (SGR 58).
    pub fn underline_color(mut self, color: Color) -> Self {
        self.props |= Props::UNDERLINE_COLOR;
        self.underline_color = color;
        self
    }

    pub fn strikethrough_spaces(self, v: bool) -> Self {
        self.set_bool(Props::STRIKETHROUGH_SPACES, v)
    }

    pub fn color_whitespace(self, v: bool) -> Self {
        self.set_bool(Props::COLOR_WHITESPACE, v)
    }

    // --- color builders ---

    pub fn foreground(mut self, c: Color) -> Self {
        self.props |= Props::FOREGROUND;
        self.fg = c;
        self
    }

    pub fn background(mut self, c: Color) -> Self {
        self.props |= Props::BACKGROUND;
        self.bg = c;
        self
    }

    pub fn margin_background(mut self, c: Color) -> Self {
        self.props |= Props::MARGIN_BG;
        self.margin_bg = c;
        self
    }

    // --- dimension builders ---

    pub fn width(mut self, w: u16) -> Self {
        self.props |= Props::WIDTH;
        self.width = w;
        self
    }

    pub fn height(mut self, h: u16) -> Self {
        self.props |= Props::HEIGHT;
        self.height = h;
        self
    }

    pub fn max_width(mut self, w: u16) -> Self {
        self.props |= Props::MAX_WIDTH;
        self.max_width = w;
        self
    }

    pub fn max_height(mut self, h: u16) -> Self {
        self.props |= Props::MAX_HEIGHT;
        self.max_height = h;
        self
    }

    // --- padding builders ---

    pub fn padding_top(mut self, v: u16) -> Self {
        self.props |= Props::PADDING_TOP;
        self.padding_top = v;
        self
    }

    pub fn padding_right(mut self, v: u16) -> Self {
        self.props |= Props::PADDING_RIGHT;
        self.padding_right = v;
        self
    }

    pub fn padding_bottom(mut self, v: u16) -> Self {
        self.props |= Props::PADDING_BOTTOM;
        self.padding_bottom = v;
        self
    }

    pub fn padding_left(mut self, v: u16) -> Self {
        self.props |= Props::PADDING_LEFT;
        self.padding_left = v;
        self
    }

    /// Set padding using CSS-like shorthand:
    /// - 1 value: all sides
    /// - 2 values: vertical, horizontal
    /// - 3 values: top, horizontal, bottom
    /// - 4 values: top, right, bottom, left
    pub fn padding(self, values: &[u16]) -> Self {
        match values.len() {
            0 => self,
            1 => self
                .padding_top(values[0])
                .padding_right(values[0])
                .padding_bottom(values[0])
                .padding_left(values[0]),
            2 => self
                .padding_top(values[0])
                .padding_right(values[1])
                .padding_bottom(values[0])
                .padding_left(values[1]),
            3 => self
                .padding_top(values[0])
                .padding_right(values[1])
                .padding_bottom(values[2])
                .padding_left(values[1]),
            _ => self
                .padding_top(values[0])
                .padding_right(values[1])
                .padding_bottom(values[2])
                .padding_left(values[3]),
        }
    }

    // --- margin builders ---

    pub fn margin_top(mut self, v: u16) -> Self {
        self.props |= Props::MARGIN_TOP;
        self.margin_top = v;
        self
    }

    pub fn margin_right(mut self, v: u16) -> Self {
        self.props |= Props::MARGIN_RIGHT;
        self.margin_right = v;
        self
    }

    pub fn margin_bottom(mut self, v: u16) -> Self {
        self.props |= Props::MARGIN_BOTTOM;
        self.margin_bottom = v;
        self
    }

    pub fn margin_left(mut self, v: u16) -> Self {
        self.props |= Props::MARGIN_LEFT;
        self.margin_left = v;
        self
    }

    /// Set margins using CSS-like shorthand (same as `padding`).
    pub fn margin(self, values: &[u16]) -> Self {
        match values.len() {
            0 => self,
            1 => self
                .margin_top(values[0])
                .margin_right(values[0])
                .margin_bottom(values[0])
                .margin_left(values[0]),
            2 => self
                .margin_top(values[0])
                .margin_right(values[1])
                .margin_bottom(values[0])
                .margin_left(values[1]),
            3 => self
                .margin_top(values[0])
                .margin_right(values[1])
                .margin_bottom(values[2])
                .margin_left(values[1]),
            _ => self
                .margin_top(values[0])
                .margin_right(values[1])
                .margin_bottom(values[2])
                .margin_left(values[3]),
        }
    }

    // --- alignment builders ---

    pub fn align_horizontal(mut self, p: Position) -> Self {
        self.props |= Props::ALIGN_H;
        self.align_h = p;
        self
    }

    pub fn align_vertical(mut self, p: Position) -> Self {
        self.props |= Props::ALIGN_V;
        self.align_v = p;
        self
    }

    pub fn align(self, h: Position, v: Position) -> Self {
        self.align_horizontal(h).align_vertical(v)
    }

    // --- border builders ---

    pub fn border_style(mut self, b: Border) -> Self {
        self.props |= Props::BORDER_STYLE;
        self.border_style = b;
        self
    }

    /// Set the border style and which sides to show.
    /// `sides` follows CSS-like shorthand:
    /// - 1 value: all sides
    /// - 2 values: top+bottom, left+right
    /// - 3 values: top, left+right, bottom
    /// - 4 values: top, right, bottom, left
    pub fn border(self, b: Border, sides: &[bool]) -> Self {
        let (top, right, bottom, left) = match sides.len() {
            0 => (true, true, true, true),
            1 => (sides[0], sides[0], sides[0], sides[0]),
            2 => (sides[0], sides[1], sides[0], sides[1]),
            3 => (sides[0], sides[1], sides[2], sides[1]),
            _ => (sides[0], sides[1], sides[2], sides[3]),
        };
        self.border_style(b)
            .border_top_side(top)
            .border_right_side(right)
            .border_bottom_side(bottom)
            .border_left_side(left)
    }

    pub fn border_top_side(self, v: bool) -> Self {
        self.set_bool(Props::BORDER_TOP, v)
    }

    pub fn border_right_side(self, v: bool) -> Self {
        self.set_bool(Props::BORDER_RIGHT, v)
    }

    pub fn border_bottom_side(self, v: bool) -> Self {
        self.set_bool(Props::BORDER_BOTTOM, v)
    }

    pub fn border_left_side(self, v: bool) -> Self {
        self.set_bool(Props::BORDER_LEFT, v)
    }

    /// Set the foreground color for all border sides.
    pub fn border_foreground(self, c: Color) -> Self {
        self.border_top_foreground(c)
            .border_right_foreground(c)
            .border_bottom_foreground(c)
            .border_left_foreground(c)
    }

    pub fn border_top_foreground(mut self, c: Color) -> Self {
        self.props |= Props::BORDER_TOP_FG;
        self.border_top_fg = c;
        self
    }

    pub fn border_right_foreground(mut self, c: Color) -> Self {
        self.props |= Props::BORDER_RIGHT_FG;
        self.border_right_fg = c;
        self
    }

    pub fn border_bottom_foreground(mut self, c: Color) -> Self {
        self.props |= Props::BORDER_BOTTOM_FG;
        self.border_bottom_fg = c;
        self
    }

    pub fn border_left_foreground(mut self, c: Color) -> Self {
        self.props |= Props::BORDER_LEFT_FG;
        self.border_left_fg = c;
        self
    }

    /// Set the background color for all border sides.
    pub fn border_background(self, c: Color) -> Self {
        self.border_top_background(c)
            .border_right_background(c)
            .border_bottom_background(c)
            .border_left_background(c)
    }

    pub fn border_top_background(mut self, c: Color) -> Self {
        self.props |= Props::BORDER_TOP_BG;
        self.border_top_bg = c;
        self
    }

    pub fn border_right_background(mut self, c: Color) -> Self {
        self.props |= Props::BORDER_RIGHT_BG;
        self.border_right_bg = c;
        self
    }

    pub fn border_bottom_background(mut self, c: Color) -> Self {
        self.props |= Props::BORDER_BOTTOM_BG;
        self.border_bottom_bg = c;
        self
    }

    pub fn border_left_background(mut self, c: Color) -> Self {
        self.props |= Props::BORDER_LEFT_BG;
        self.border_left_bg = c;
        self
    }

    // --- misc builders ---

    pub fn inline(self, v: bool) -> Self {
        self.set_bool(Props::INLINE, v)
    }

    pub fn tab_width(mut self, w: i8) -> Self {
        self.props |= Props::TAB_WIDTH;
        self.tab_width = w;
        self
    }

    /// Set a post-render transform function.
    pub fn transform(mut self, f: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
        self.props |= Props::TRANSFORM;
        self.transform = Some(Arc::new(f));
        self
    }

    /// Set an OSC 8 hyperlink URL on the rendered text.
    pub fn hyperlink(mut self, url: &str) -> Self {
        self.props |= Props::HYPERLINK;
        self.hyperlink = Some(url.to_string());
        self
    }

    /// Copy unset properties from `other` into `self`.
    /// Margin and padding are NOT inherited.
    pub fn inherit(mut self, other: &Style) -> Self {
        // Iterate through all property bits. For each one that is set in `other`
        // but not in `self`, copy it over (except margin/padding).
        let skip = Props::MARGIN_TOP
            | Props::MARGIN_RIGHT
            | Props::MARGIN_BOTTOM
            | Props::MARGIN_LEFT
            | Props::PADDING_TOP
            | Props::PADDING_RIGHT
            | Props::PADDING_BOTTOM
            | Props::PADDING_LEFT;

        // For each bit in Props that `other` has set but `self` does not:
        let to_inherit = other.props & !self.props & !skip;

        // Bool attrs: copy from other.attrs for the inherited bits that are bool attrs
        let bool_inherit = to_inherit & BOOL_ATTRS;
        self.attrs |= other.attrs & bool_inherit;

        // Colors
        if to_inherit.contains(Props::FOREGROUND) {
            self.fg = other.fg;
        }
        if to_inherit.contains(Props::BACKGROUND) {
            self.bg = other.bg;
            // Background also inherits to margin bg if margin bg isn't set
            if !self.is_set(Props::MARGIN_BG) && !other.is_set(Props::MARGIN_BG) {
                self.margin_bg = other.bg;
                // Don't set the flag though — it's implicit
            }
        }
        if to_inherit.contains(Props::MARGIN_BG) {
            self.margin_bg = other.margin_bg;
        }
        if to_inherit.contains(Props::BORDER_TOP_FG) {
            self.border_top_fg = other.border_top_fg;
        }
        if to_inherit.contains(Props::BORDER_RIGHT_FG) {
            self.border_right_fg = other.border_right_fg;
        }
        if to_inherit.contains(Props::BORDER_BOTTOM_FG) {
            self.border_bottom_fg = other.border_bottom_fg;
        }
        if to_inherit.contains(Props::BORDER_LEFT_FG) {
            self.border_left_fg = other.border_left_fg;
        }
        if to_inherit.contains(Props::BORDER_TOP_BG) {
            self.border_top_bg = other.border_top_bg;
        }
        if to_inherit.contains(Props::BORDER_RIGHT_BG) {
            self.border_right_bg = other.border_right_bg;
        }
        if to_inherit.contains(Props::BORDER_BOTTOM_BG) {
            self.border_bottom_bg = other.border_bottom_bg;
        }
        if to_inherit.contains(Props::BORDER_LEFT_BG) {
            self.border_left_bg = other.border_left_bg;
        }

        // Underline
        if to_inherit.contains(Props::UNDERLINE_STYLE) {
            self.underline_style = other.underline_style;
        }
        if to_inherit.contains(Props::UNDERLINE_COLOR) {
            self.underline_color = other.underline_color;
        }

        // Dimensions
        if to_inherit.contains(Props::WIDTH) {
            self.width = other.width;
        }
        if to_inherit.contains(Props::HEIGHT) {
            self.height = other.height;
        }
        if to_inherit.contains(Props::MAX_WIDTH) {
            self.max_width = other.max_width;
        }
        if to_inherit.contains(Props::MAX_HEIGHT) {
            self.max_height = other.max_height;
        }

        // Alignment
        if to_inherit.contains(Props::ALIGN_H) {
            self.align_h = other.align_h;
        }
        if to_inherit.contains(Props::ALIGN_V) {
            self.align_v = other.align_v;
        }

        // Border style
        if to_inherit.contains(Props::BORDER_STYLE) {
            self.border_style = other.border_style.clone();
        }

        // Tab width
        if to_inherit.contains(Props::TAB_WIDTH) {
            self.tab_width = other.tab_width;
        }

        self.props |= to_inherit;
        self
    }

    // --- getters ---

    pub fn get_bold(&self) -> bool { self.get_bool(Props::BOLD) }
    pub fn get_italic(&self) -> bool { self.get_bool(Props::ITALIC) }
    pub fn get_underline(&self) -> bool { self.get_bool(Props::UNDERLINE) }
    pub fn get_strikethrough(&self) -> bool { self.get_bool(Props::STRIKETHROUGH) }
    pub fn get_reverse(&self) -> bool { self.get_bool(Props::REVERSE) }
    pub fn get_blink(&self) -> bool { self.get_bool(Props::BLINK) }
    pub fn get_faint(&self) -> bool { self.get_bool(Props::FAINT) }
    pub fn get_underline_spaces(&self) -> bool { self.get_bool(Props::UNDERLINE_SPACES) }
    pub fn get_strikethrough_spaces(&self) -> bool { self.get_bool(Props::STRIKETHROUGH_SPACES) }
    pub fn get_inline(&self) -> bool { self.get_bool(Props::INLINE) }
    pub fn get_foreground(&self) -> Color { if self.is_set(Props::FOREGROUND) { self.fg } else { Color::NoColor } }
    pub fn get_background(&self) -> Color { if self.is_set(Props::BACKGROUND) { self.bg } else { Color::NoColor } }
    pub fn get_margin_background(&self) -> Color { if self.is_set(Props::MARGIN_BG) { self.margin_bg } else { Color::NoColor } }
    pub fn get_underline_style(&self) -> UnderlineStyle { if self.is_set(Props::UNDERLINE_STYLE) { self.underline_style } else { UnderlineStyle::None } }
    pub fn get_underline_color(&self) -> Color { if self.is_set(Props::UNDERLINE_COLOR) { self.underline_color } else { Color::NoColor } }
    pub fn get_width(&self) -> u16 { if self.is_set(Props::WIDTH) { self.width } else { 0 } }
    pub fn get_height(&self) -> u16 { if self.is_set(Props::HEIGHT) { self.height } else { 0 } }
    pub fn get_max_width(&self) -> u16 { if self.is_set(Props::MAX_WIDTH) { self.max_width } else { 0 } }
    pub fn get_max_height(&self) -> u16 { if self.is_set(Props::MAX_HEIGHT) { self.max_height } else { 0 } }
    pub fn get_padding_top(&self) -> u16 { if self.is_set(Props::PADDING_TOP) { self.padding_top } else { 0 } }
    pub fn get_padding_right(&self) -> u16 { if self.is_set(Props::PADDING_RIGHT) { self.padding_right } else { 0 } }
    pub fn get_padding_bottom(&self) -> u16 { if self.is_set(Props::PADDING_BOTTOM) { self.padding_bottom } else { 0 } }
    pub fn get_padding_left(&self) -> u16 { if self.is_set(Props::PADDING_LEFT) { self.padding_left } else { 0 } }
    pub fn get_margin_top(&self) -> u16 { if self.is_set(Props::MARGIN_TOP) { self.margin_top } else { 0 } }
    pub fn get_margin_right(&self) -> u16 { if self.is_set(Props::MARGIN_RIGHT) { self.margin_right } else { 0 } }
    pub fn get_margin_bottom(&self) -> u16 { if self.is_set(Props::MARGIN_BOTTOM) { self.margin_bottom } else { 0 } }
    pub fn get_margin_left(&self) -> u16 { if self.is_set(Props::MARGIN_LEFT) { self.margin_left } else { 0 } }
    pub fn get_align_horizontal(&self) -> Position { if self.is_set(Props::ALIGN_H) { self.align_h } else { Position::LEFT } }
    pub fn get_align_vertical(&self) -> Position { if self.is_set(Props::ALIGN_V) { self.align_v } else { Position::TOP } }
    pub fn get_border_style(&self) -> &Border { &self.border_style }
    pub fn get_border_top_foreground(&self) -> Color { if self.is_set(Props::BORDER_TOP_FG) { self.border_top_fg } else { Color::NoColor } }
    pub fn get_border_right_foreground(&self) -> Color { if self.is_set(Props::BORDER_RIGHT_FG) { self.border_right_fg } else { Color::NoColor } }
    pub fn get_border_bottom_foreground(&self) -> Color { if self.is_set(Props::BORDER_BOTTOM_FG) { self.border_bottom_fg } else { Color::NoColor } }
    pub fn get_border_left_foreground(&self) -> Color { if self.is_set(Props::BORDER_LEFT_FG) { self.border_left_fg } else { Color::NoColor } }
    pub fn get_border_top_background(&self) -> Color { if self.is_set(Props::BORDER_TOP_BG) { self.border_top_bg } else { Color::NoColor } }
    pub fn get_border_right_background(&self) -> Color { if self.is_set(Props::BORDER_RIGHT_BG) { self.border_right_bg } else { Color::NoColor } }
    pub fn get_border_bottom_background(&self) -> Color { if self.is_set(Props::BORDER_BOTTOM_BG) { self.border_bottom_bg } else { Color::NoColor } }
    pub fn get_border_left_background(&self) -> Color { if self.is_set(Props::BORDER_LEFT_BG) { self.border_left_bg } else { Color::NoColor } }
    pub fn get_tab_width(&self) -> i8 { if self.is_set(Props::TAB_WIDTH) { self.tab_width } else { 4 } }

    /// Total frame size (margins + padding + borders) as (width, height).
    pub fn frame_size(&self) -> (usize, usize) {
        (self.get_horizontal_frame_size(), self.get_vertical_frame_size())
    }

    // --- unset methods ---

    fn unset_prop(mut self, prop: Props) -> Self { self.props -= prop; self.attrs -= prop; self }

    pub fn unset_bold(self) -> Self { self.unset_prop(Props::BOLD) }
    pub fn unset_italic(self) -> Self { self.unset_prop(Props::ITALIC) }
    pub fn unset_underline(self) -> Self { self.unset_prop(Props::UNDERLINE) }
    pub fn unset_strikethrough(self) -> Self { self.unset_prop(Props::STRIKETHROUGH) }
    pub fn unset_reverse(self) -> Self { self.unset_prop(Props::REVERSE) }
    pub fn unset_blink(self) -> Self { self.unset_prop(Props::BLINK) }
    pub fn unset_faint(self) -> Self { self.unset_prop(Props::FAINT) }
    pub fn unset_inline(self) -> Self { self.unset_prop(Props::INLINE) }
    pub fn unset_foreground(mut self) -> Self { self.props -= Props::FOREGROUND; self.fg = Color::NoColor; self }
    pub fn unset_background(mut self) -> Self { self.props -= Props::BACKGROUND; self.bg = Color::NoColor; self }
    pub fn unset_width(mut self) -> Self { self.props -= Props::WIDTH; self.width = 0; self }
    pub fn unset_height(mut self) -> Self { self.props -= Props::HEIGHT; self.height = 0; self }
    pub fn unset_max_width(mut self) -> Self { self.props -= Props::MAX_WIDTH; self.max_width = 0; self }
    pub fn unset_max_height(mut self) -> Self { self.props -= Props::MAX_HEIGHT; self.max_height = 0; self }
    pub fn unset_padding_top(mut self) -> Self { self.props -= Props::PADDING_TOP; self.padding_top = 0; self }
    pub fn unset_padding_right(mut self) -> Self { self.props -= Props::PADDING_RIGHT; self.padding_right = 0; self }
    pub fn unset_padding_bottom(mut self) -> Self { self.props -= Props::PADDING_BOTTOM; self.padding_bottom = 0; self }
    pub fn unset_padding_left(mut self) -> Self { self.props -= Props::PADDING_LEFT; self.padding_left = 0; self }
    pub fn unset_margin_top(mut self) -> Self { self.props -= Props::MARGIN_TOP; self.margin_top = 0; self }
    pub fn unset_margin_right(mut self) -> Self { self.props -= Props::MARGIN_RIGHT; self.margin_right = 0; self }
    pub fn unset_margin_bottom(mut self) -> Self { self.props -= Props::MARGIN_BOTTOM; self.margin_bottom = 0; self }
    pub fn unset_margin_left(mut self) -> Self { self.props -= Props::MARGIN_LEFT; self.margin_left = 0; self }
    pub fn unset_border_style(mut self) -> Self { self.props -= Props::BORDER_STYLE; self.border_style = NO_BORDER; self }

    // --- size helpers ---

    /// Whether the border style is set but no individual sides are set
    /// (meaning all sides should render by default).
    pub(crate) fn is_border_style_set_without_sides(&self) -> bool {
        let has_style = self.is_set(Props::BORDER_STYLE) && self.border_style != NO_BORDER;
        let any_side = self.is_set(Props::BORDER_TOP)
            | self.is_set(Props::BORDER_RIGHT)
            | self.is_set(Props::BORDER_BOTTOM)
            | self.is_set(Props::BORDER_LEFT);
        has_style && !any_side
    }

    pub fn get_border_top_size(&self) -> usize {
        if self.is_border_style_set_without_sides() {
            return 1;
        }
        if !self.get_bool(Props::BORDER_TOP) {
            return 0;
        }
        self.border_style.get_top_size()
    }

    pub fn get_border_bottom_size(&self) -> usize {
        if self.is_border_style_set_without_sides() {
            return 1;
        }
        if !self.get_bool(Props::BORDER_BOTTOM) {
            return 0;
        }
        self.border_style.get_bottom_size()
    }

    pub fn get_border_left_size(&self) -> usize {
        if self.is_border_style_set_without_sides() {
            return 1;
        }
        if !self.get_bool(Props::BORDER_LEFT) {
            return 0;
        }
        self.border_style.get_left_size()
    }

    pub fn get_border_right_size(&self) -> usize {
        if self.is_border_style_set_without_sides() {
            return 1;
        }
        if !self.get_bool(Props::BORDER_RIGHT) {
            return 0;
        }
        self.border_style.get_right_size()
    }

    pub fn get_horizontal_border_size(&self) -> usize {
        self.get_border_left_size() + self.get_border_right_size()
    }

    pub fn get_vertical_border_size(&self) -> usize {
        self.get_border_top_size() + self.get_border_bottom_size()
    }

    pub fn get_horizontal_padding(&self) -> usize {
        self.padding_left as usize + self.padding_right as usize
    }

    pub fn get_vertical_padding(&self) -> usize {
        self.padding_top as usize + self.padding_bottom as usize
    }

    pub fn get_horizontal_margins(&self) -> usize {
        self.margin_left as usize + self.margin_right as usize
    }

    pub fn get_vertical_margins(&self) -> usize {
        self.margin_top as usize + self.margin_bottom as usize
    }

    pub fn get_horizontal_frame_size(&self) -> usize {
        self.get_horizontal_margins() + self.get_horizontal_padding() + self.get_horizontal_border_size()
    }

    pub fn get_vertical_frame_size(&self) -> usize {
        self.get_vertical_margins() + self.get_vertical_padding() + self.get_vertical_border_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_style() {
        let s = Style::new();
        assert_eq!(s.props, Props::empty());
        assert!(!s.get_bool(Props::BOLD));
    }

    #[test]
    fn test_bold() {
        let s = Style::new().bold(true);
        assert!(s.get_bool(Props::BOLD));
        assert!(s.is_set(Props::BOLD));
    }

    #[test]
    fn test_bold_false() {
        let s = Style::new().bold(false);
        assert!(!s.get_bool(Props::BOLD));
        assert!(s.is_set(Props::BOLD)); // set, but to false
    }

    #[test]
    fn test_padding_shorthand() {
        let s = Style::new().padding(&[1, 2]);
        assert_eq!(s.padding_top, 1);
        assert_eq!(s.padding_right, 2);
        assert_eq!(s.padding_bottom, 1);
        assert_eq!(s.padding_left, 2);
    }

    #[test]
    fn test_inherit() {
        let parent = Style::new().bold(true).foreground(Color::Rgb { r: 255, g: 0, b: 0 });
        let child = Style::new().italic(true).inherit(&parent);
        assert!(child.get_bool(Props::BOLD));
        assert!(child.get_bool(Props::ITALIC));
        assert_eq!(child.fg, Color::Rgb { r: 255, g: 0, b: 0 });
    }

    #[test]
    fn test_inherit_no_overwrite() {
        let parent = Style::new().foreground(Color::Rgb { r: 255, g: 0, b: 0 });
        let child = Style::new()
            .foreground(Color::Rgb { r: 0, g: 255, b: 0 })
            .inherit(&parent);
        // Child's fg should remain green
        assert_eq!(child.fg, Color::Rgb { r: 0, g: 255, b: 0 });
    }

    #[test]
    fn test_inherit_skips_margin_padding() {
        let parent = Style::new().margin(&[5]).padding(&[3]);
        let child = Style::new().inherit(&parent);
        assert_eq!(child.margin_top, 0);
        assert_eq!(child.padding_top, 0);
    }

    #[test]
    fn test_border_sizes() {
        let s = Style::new().border(crate::border::NORMAL_BORDER, &[true]);
        assert_eq!(s.get_border_top_size(), 1);
        assert_eq!(s.get_border_bottom_size(), 1);
        assert_eq!(s.get_border_left_size(), 1);
        assert_eq!(s.get_border_right_size(), 1);
    }

    #[test]
    fn test_border_style_without_sides() {
        let s = Style::new().border_style(crate::border::NORMAL_BORDER);
        // When border style is set without sides, all sides should render
        assert!(s.is_border_style_set_without_sides());
        assert_eq!(s.get_border_top_size(), 1);
    }
}
