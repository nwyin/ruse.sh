use std::fmt;

/// Terminal color profile representing the level of color support.
///
/// Ordered from least capable to most capable, so `max()` can be used
/// to combine detected profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
#[derive(Default)]
pub enum Profile {
    /// No terminal support.
    #[default]
    NoTty = 0,
    /// Terminal present but no color support.
    Ascii = 1,
    /// 16 colors (4-bit).
    Ansi = 2,
    /// 256 colors (8-bit).
    Ansi256 = 3,
    /// 16.7 million colors (24-bit).
    TrueColor = 4,
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Profile::TrueColor => write!(f, "TrueColor"),
            Profile::Ansi256 => write!(f, "ANSI256"),
            Profile::Ansi => write!(f, "ANSI"),
            Profile::Ascii => write!(f, "Ascii"),
            Profile::NoTty => write!(f, "NoTTY"),
        }
    }
}
