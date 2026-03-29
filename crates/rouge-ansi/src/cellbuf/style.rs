use std::fmt::Write;

/// Bitmask for text attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AttrMask(u8);

impl AttrMask {
    pub const BOLD: Self = Self(1 << 0);
    pub const FAINT: Self = Self(1 << 1);
    pub const ITALIC: Self = Self(1 << 2);
    pub const SLOW_BLINK: Self = Self(1 << 3);
    pub const RAPID_BLINK: Self = Self(1 << 4);
    pub const REVERSE: Self = Self(1 << 5);
    pub const CONCEAL: Self = Self(1 << 6);
    pub const STRIKETHROUGH: Self = Self(1 << 7);

    pub const NONE: Self = Self(0);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn set(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn unset(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn bits(self) -> u8 {
        self.0
    }
}

impl std::ops::BitOr for AttrMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Underline style variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

/// Cell-level style for the cell buffer.
///
/// Uses `Option<(u8, u8, u8)>` for colors to keep it lightweight.
/// `None` means the terminal default color.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CellStyle {
    pub fg: Option<(u8, u8, u8)>,
    pub bg: Option<(u8, u8, u8)>,
    pub ul: Option<(u8, u8, u8)>,
    pub attrs: AttrMask,
    pub ul_style: UnderlineStyle,
}

impl CellStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.fg.is_none()
            && self.bg.is_none()
            && self.ul.is_none()
            && self.attrs.is_empty()
            && self.ul_style == UnderlineStyle::None
    }

    /// Whether this style only affects visible content (not blank cells).
    /// Returns true if the style has no background, reverse, or underline
    /// that would be visible on a blank cell.
    pub fn is_clear(&self) -> bool {
        self.bg.is_none()
            && !self.attrs.contains(AttrMask::REVERSE)
            && self.ul_style == UnderlineStyle::None
    }

    /// Generate the full SGR sequence to apply this style from a reset state.
    pub fn sequence(&self) -> String {
        if self.is_empty() {
            return String::new();
        }
        let mut s = String::with_capacity(32);
        s.push_str("\x1b[0");
        self.write_attrs(&mut s);
        s.push('m');
        s
    }

    /// Generate the minimal SGR to transform from `old` to `self`.
    /// Only writes the differences.
    pub fn diff_sequence(&self, old: &CellStyle) -> String {
        if self == old {
            return String::new();
        }

        // If new style is empty/default, just reset
        if self.is_empty() {
            return "\x1b[0m".to_string();
        }

        // If old style is empty/default, we can just add what's needed
        // (no need for a full reset prefix)

        // Check if we need a full reset (attribute removed)
        let needs_reset = self.needs_reset_from(old);
        if needs_reset {
            return self.sequence();
        }

        let mut s = String::with_capacity(32);
        let mut first = true;

        macro_rules! emit {
            ($code:expr) => {
                if first {
                    s.push_str("\x1b[");
                    first = false;
                } else {
                    s.push(';');
                }
                let _ = write!(s, "{}", $code);
            };
        }

        // Diff attributes
        let added = AttrMask(self.attrs.bits() & !old.attrs.bits());
        if added.contains(AttrMask::BOLD) {
            emit!("1");
        }
        if added.contains(AttrMask::FAINT) {
            emit!("2");
        }
        if added.contains(AttrMask::ITALIC) {
            emit!("3");
        }
        if added.contains(AttrMask::SLOW_BLINK) {
            emit!("5");
        }
        if added.contains(AttrMask::RAPID_BLINK) {
            emit!("6");
        }
        if added.contains(AttrMask::REVERSE) {
            emit!("7");
        }
        if added.contains(AttrMask::CONCEAL) {
            emit!("8");
        }
        if added.contains(AttrMask::STRIKETHROUGH) {
            emit!("9");
        }

        // Diff underline style
        if self.ul_style != old.ul_style {
            match self.ul_style {
                UnderlineStyle::None => {
                    emit!("24");
                }
                UnderlineStyle::Single => {
                    emit!("4");
                }
                UnderlineStyle::Double => {
                    emit!("21");
                }
                UnderlineStyle::Curly => {
                    emit!("4:3");
                }
                UnderlineStyle::Dotted => {
                    emit!("4:4");
                }
                UnderlineStyle::Dashed => {
                    emit!("4:5");
                }
            }
        }

        // Diff colors
        if self.fg != old.fg {
            match self.fg {
                Some((r, g, b)) => {
                    emit!(format_args!("38;2;{};{};{}", r, g, b));
                }
                None => {
                    emit!("39");
                }
            }
        }
        if self.bg != old.bg {
            match self.bg {
                Some((r, g, b)) => {
                    emit!(format_args!("48;2;{};{};{}", r, g, b));
                }
                None => {
                    emit!("49");
                }
            }
        }
        if self.ul != old.ul {
            match self.ul {
                Some((r, g, b)) => {
                    emit!(format_args!("58;2;{};{};{}", r, g, b));
                }
                None => {
                    emit!("59");
                }
            }
        }

        if first {
            // Nothing changed
            String::new()
        } else {
            s.push('m');
            s
        }
    }

    fn write_attrs(&self, s: &mut String) {
        if self.attrs.contains(AttrMask::BOLD) {
            s.push_str(";1");
        }
        if self.attrs.contains(AttrMask::FAINT) {
            s.push_str(";2");
        }
        if self.attrs.contains(AttrMask::ITALIC) {
            s.push_str(";3");
        }
        match self.ul_style {
            UnderlineStyle::None => {}
            UnderlineStyle::Single => s.push_str(";4"),
            UnderlineStyle::Double => s.push_str(";21"),
            UnderlineStyle::Curly => s.push_str(";4:3"),
            UnderlineStyle::Dotted => s.push_str(";4:4"),
            UnderlineStyle::Dashed => s.push_str(";4:5"),
        }
        if self.attrs.contains(AttrMask::SLOW_BLINK) {
            s.push_str(";5");
        }
        if self.attrs.contains(AttrMask::RAPID_BLINK) {
            s.push_str(";6");
        }
        if self.attrs.contains(AttrMask::REVERSE) {
            s.push_str(";7");
        }
        if self.attrs.contains(AttrMask::CONCEAL) {
            s.push_str(";8");
        }
        if self.attrs.contains(AttrMask::STRIKETHROUGH) {
            s.push_str(";9");
        }
        if let Some((r, g, b)) = self.fg {
            let _ = write!(s, ";38;2;{};{};{}", r, g, b);
        }
        if let Some((r, g, b)) = self.bg {
            let _ = write!(s, ";48;2;{};{};{}", r, g, b);
        }
        if let Some((r, g, b)) = self.ul {
            let _ = write!(s, ";58;2;{};{};{}", r, g, b);
        }
    }

    /// Whether transforming from `old` to `self` requires a full reset.
    /// This happens when an attribute is removed (can't un-bold without reset).
    fn needs_reset_from(&self, old: &CellStyle) -> bool {
        let removed = AttrMask(old.attrs.bits() & !self.attrs.bits());
        !removed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_style() {
        let s = CellStyle::new();
        assert!(s.is_empty());
        assert_eq!(s.sequence(), "");
    }

    #[test]
    fn test_bold_sequence() {
        let mut s = CellStyle::new();
        s.attrs.set(AttrMask::BOLD);
        assert_eq!(s.sequence(), "\x1b[0;1m");
    }

    #[test]
    fn test_fg_color() {
        let mut s = CellStyle::new();
        s.fg = Some((255, 0, 128));
        assert_eq!(s.sequence(), "\x1b[0;38;2;255;0;128m");
    }

    #[test]
    fn test_diff_add_bold() {
        let old = CellStyle::new();
        let mut new = CellStyle::new();
        new.attrs.set(AttrMask::BOLD);
        assert_eq!(new.diff_sequence(&old), "\x1b[1m");
    }

    #[test]
    fn test_diff_remove_bold_needs_reset() {
        let mut old = CellStyle::new();
        old.attrs.set(AttrMask::BOLD);
        let new = CellStyle::new();
        // Removing bold requires full reset
        assert_eq!(new.diff_sequence(&old), "\x1b[0m");
    }

    #[test]
    fn test_diff_no_change() {
        let mut s = CellStyle::new();
        s.attrs.set(AttrMask::BOLD);
        assert_eq!(s.diff_sequence(&s), "");
    }

    #[test]
    fn test_diff_change_fg() {
        let mut old = CellStyle::new();
        old.fg = Some((255, 0, 0));
        let mut new = CellStyle::new();
        new.fg = Some((0, 255, 0));
        assert_eq!(new.diff_sequence(&old), "\x1b[38;2;0;255;0m");
    }

    #[test]
    fn test_underline_curly() {
        let mut s = CellStyle::new();
        s.ul_style = UnderlineStyle::Curly;
        assert_eq!(s.sequence(), "\x1b[0;4:3m");
    }
}
