use std::fmt;

/// A builder that accumulates SGR (Select Graphic Rendition) parameters
/// and produces ANSI escape sequences.
#[derive(Clone, Debug, Default)]
pub struct SgrStyle {
    params: String,
}

impl SgrStyle {
    pub fn new() -> Self {
        Self {
            params: String::new(),
        }
    }

    fn push_param(mut self, param: &str) -> Self {
        if !self.params.is_empty() {
            self.params.push(';');
        }
        self.params.push_str(param);
        self
    }

    fn push_num(self, n: u8) -> Self {
        self.push_param(&n.to_string())
    }

    // Attributes
    pub fn bold(self) -> Self {
        self.push_num(1)
    }

    pub fn faint(self) -> Self {
        self.push_num(2)
    }

    pub fn italic(self) -> Self {
        self.push_num(3)
    }

    pub fn underline(self) -> Self {
        self.push_num(4)
    }

    pub fn blink(self) -> Self {
        self.push_num(5)
    }

    pub fn reverse(self) -> Self {
        self.push_num(7)
    }

    pub fn strikethrough(self) -> Self {
        self.push_num(9)
    }

    // Underline styles
    pub fn double_underline(self) -> Self {
        self.push_num(21)
    }

    pub fn curly_underline(self) -> Self {
        self.push_param("4:3")
    }

    pub fn dotted_underline(self) -> Self {
        self.push_param("4:4")
    }

    pub fn dashed_underline(self) -> Self {
        self.push_param("4:5")
    }

    // Advanced attributes
    pub fn overline(self) -> Self {
        self.push_num(53)
    }

    pub fn superscript(self) -> Self {
        self.push_num(73)
    }

    pub fn subscript(self) -> Self {
        self.push_num(74)
    }

    /// Select alternate font (0=default, 1-9=alternate).
    pub fn font(self, n: u8) -> Self {
        self.push_num(10 + n.min(9))
    }

    /// Normal intensity (resets bold and faint).
    pub fn normal_intensity(self) -> Self {
        self.push_num(22)
    }

    // Foreground colors
    pub fn fg_basic(self, color: u8) -> Self {
        let code = if color < 8 {
            30 + color
        } else {
            90 + (color - 8)
        };
        self.push_num(code)
    }

    pub fn fg_256(self, color: u8) -> Self {
        self.push_param(&format!("38;5;{color}"))
    }

    pub fn fg_rgb(self, r: u8, g: u8, b: u8) -> Self {
        self.push_param(&format!("38;2;{r};{g};{b}"))
    }

    // Background colors
    pub fn bg_basic(self, color: u8) -> Self {
        let code = if color < 8 {
            40 + color
        } else {
            100 + (color - 8)
        };
        self.push_num(code)
    }

    pub fn bg_256(self, color: u8) -> Self {
        self.push_param(&format!("48;5;{color}"))
    }

    pub fn bg_rgb(self, r: u8, g: u8, b: u8) -> Self {
        self.push_param(&format!("48;2;{r};{g};{b}"))
    }

    // Underline colors
    pub fn ul_256(self, color: u8) -> Self {
        self.push_param(&format!("58;5;{color}"))
    }

    pub fn ul_rgb(self, r: u8, g: u8, b: u8) -> Self {
        self.push_param(&format!("58;2;{r};{g};{b}"))
    }

    // Output
    pub fn open(&self) -> String {
        format!("\x1b[{}m", self.params)
    }

    pub fn close() -> &'static str {
        "\x1b[0m"
    }

    pub fn styled(&self, s: &str) -> String {
        if self.is_empty() {
            return s.to_string();
        }
        format!("{}{}{}", self.open(), s, Self::close())
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }
}

impl fmt::Display for SgrStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            Ok(())
        } else {
            write!(f, "\x1b[{}m", self.params)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let s = SgrStyle::new();
        assert!(s.is_empty());
        assert_eq!(s.styled("hello"), "hello");
    }

    #[test]
    fn test_bold() {
        let s = SgrStyle::new().bold();
        assert_eq!(s.open(), "\x1b[1m");
        assert_eq!(SgrStyle::close(), "\x1b[0m");
        assert_eq!(s.styled("hi"), "\x1b[1mhi\x1b[0m");
    }

    #[test]
    fn test_combined() {
        let s = SgrStyle::new().bold().italic().underline();
        assert_eq!(s.open(), "\x1b[1;3;4m");
    }

    #[test]
    fn test_fg_basic_low() {
        let s = SgrStyle::new().fg_basic(1); // red
        assert_eq!(s.open(), "\x1b[31m");
    }

    #[test]
    fn test_fg_basic_high() {
        let s = SgrStyle::new().fg_basic(9); // bright red
        assert_eq!(s.open(), "\x1b[91m");
    }

    #[test]
    fn test_bg_basic_low() {
        let s = SgrStyle::new().bg_basic(2); // green bg
        assert_eq!(s.open(), "\x1b[42m");
    }

    #[test]
    fn test_bg_basic_high() {
        let s = SgrStyle::new().bg_basic(10); // bright green bg
        assert_eq!(s.open(), "\x1b[102m");
    }

    #[test]
    fn test_fg_256() {
        let s = SgrStyle::new().fg_256(196);
        assert_eq!(s.open(), "\x1b[38;5;196m");
    }

    #[test]
    fn test_fg_rgb() {
        let s = SgrStyle::new().fg_rgb(255, 128, 0);
        assert_eq!(s.open(), "\x1b[38;2;255;128;0m");
    }

    #[test]
    fn test_bg_256() {
        let s = SgrStyle::new().bg_256(42);
        assert_eq!(s.open(), "\x1b[48;5;42m");
    }

    #[test]
    fn test_bg_rgb() {
        let s = SgrStyle::new().bg_rgb(10, 20, 30);
        assert_eq!(s.open(), "\x1b[48;2;10;20;30m");
    }

    #[test]
    fn test_ul_256() {
        let s = SgrStyle::new().ul_256(100);
        assert_eq!(s.open(), "\x1b[58;5;100m");
    }

    #[test]
    fn test_ul_rgb() {
        let s = SgrStyle::new().ul_rgb(1, 2, 3);
        assert_eq!(s.open(), "\x1b[58;2;1;2;3m");
    }

    #[test]
    fn test_curly_underline() {
        let s = SgrStyle::new().curly_underline();
        assert_eq!(s.open(), "\x1b[4:3m");
    }

    #[test]
    fn test_dotted_underline() {
        let s = SgrStyle::new().dotted_underline();
        assert_eq!(s.open(), "\x1b[4:4m");
    }

    #[test]
    fn test_dashed_underline() {
        let s = SgrStyle::new().dashed_underline();
        assert_eq!(s.open(), "\x1b[4:5m");
    }

    #[test]
    fn test_double_underline() {
        let s = SgrStyle::new().double_underline();
        assert_eq!(s.open(), "\x1b[21m");
    }

    #[test]
    fn test_all_attributes() {
        let s = SgrStyle::new()
            .bold()
            .faint()
            .italic()
            .underline()
            .blink()
            .reverse()
            .strikethrough();
        assert_eq!(s.open(), "\x1b[1;2;3;4;5;7;9m");
    }

    #[test]
    fn test_display() {
        let s = SgrStyle::new().bold();
        assert_eq!(format!("{s}"), "\x1b[1m");

        let empty = SgrStyle::new();
        assert_eq!(format!("{empty}"), "");
    }
}
