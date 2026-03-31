/// Image protocol supported by the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageProtocol {
    /// Kitty graphics protocol (most capable).
    Kitty,
    /// Sixel graphics (wide terminal support: xterm, foot, WezTerm).
    Sixel,
    /// iTerm2 inline images (OSC 1337).
    Iterm2,
    /// Unicode half-block fallback (works everywhere with Unicode + TrueColor).
    Mosaic,
    /// No image support detected.
    None,
}

/// Detect the best image protocol for the current terminal based on
/// environment variables. Does not perform terminal queries.
pub fn detect_image_protocol() -> ImageProtocol {
    detect_from_env(
        std::env::var("TERM").ok().as_deref(),
        std::env::var("TERM_PROGRAM").ok().as_deref(),
        std::env::var("COLORTERM").ok().as_deref(),
    )
}

/// Detect image protocol from specific env var values (testable).
pub fn detect_from_env(
    term: Option<&str>,
    term_program: Option<&str>,
    colorterm: Option<&str>,
) -> ImageProtocol {
    let term = term.unwrap_or("");
    let term_program = term_program.unwrap_or("");

    // Kitty
    if term.contains("kitty") || term_program.eq_ignore_ascii_case("kitty") {
        return ImageProtocol::Kitty;
    }

    // iTerm2
    if term_program.eq_ignore_ascii_case("iTerm.app") || term_program.eq_ignore_ascii_case("iTerm2") {
        return ImageProtocol::Iterm2;
    }

    // WezTerm supports Kitty graphics + Sixel
    if term.contains("wezterm") || term_program.eq_ignore_ascii_case("WezTerm") {
        return ImageProtocol::Kitty;
    }

    // Terminals known to support Sixel
    let sixel_terms = ["foot", "mlterm", "yaft", "contour"];
    for t in &sixel_terms {
        if term.contains(t) {
            return ImageProtocol::Sixel;
        }
    }

    // xterm supports Sixel when compiled with --enable-sixel-graphics
    if term.starts_with("xterm") && !term.contains("kitty") && !term.contains("ghostty") {
        return ImageProtocol::Sixel;
    }

    // If we have TrueColor support, mosaic is always available
    let has_truecolor = colorterm
        .map(|c| {
            let c = c.to_lowercase();
            c == "truecolor" || c == "24bit"
        })
        .unwrap_or(false)
        || term.ends_with("direct")
        || ["alacritty", "rio", "ghostty"].iter().any(|t| term.contains(t));

    if has_truecolor {
        return ImageProtocol::Mosaic;
    }

    ImageProtocol::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kitty() {
        assert_eq!(
            detect_from_env(Some("xterm-kitty"), None, None),
            ImageProtocol::Kitty
        );
    }

    #[test]
    fn test_iterm() {
        assert_eq!(
            detect_from_env(Some("xterm-256color"), Some("iTerm.app"), None),
            ImageProtocol::Iterm2
        );
    }

    #[test]
    fn test_wezterm() {
        assert_eq!(
            detect_from_env(Some("wezterm"), None, None),
            ImageProtocol::Kitty
        );
    }

    #[test]
    fn test_foot_sixel() {
        assert_eq!(
            detect_from_env(Some("foot"), None, None),
            ImageProtocol::Sixel
        );
    }

    #[test]
    fn test_xterm_sixel() {
        assert_eq!(
            detect_from_env(Some("xterm-256color"), None, None),
            ImageProtocol::Sixel
        );
    }

    #[test]
    fn test_ghostty_mosaic() {
        assert_eq!(
            detect_from_env(Some("xterm-ghostty"), None, None),
            ImageProtocol::Mosaic
        );
    }

    #[test]
    fn test_truecolor_mosaic() {
        assert_eq!(
            detect_from_env(Some("screen"), None, Some("truecolor")),
            ImageProtocol::Mosaic
        );
    }

    #[test]
    fn test_unknown_none() {
        assert_eq!(
            detect_from_env(Some("dumb"), None, None),
            ImageProtocol::None
        );
    }
}
