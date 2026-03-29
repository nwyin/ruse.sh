use std::collections::HashMap;
use std::env;

use crate::profile::Profile;

impl Profile {
    /// Detect terminal color profile from the current process environment.
    pub fn detect_env() -> Self {
        let vars: Vec<(String, String)> = env::vars().collect();
        let env_map: Vec<(&str, &str)> = vars.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        Self::from_env(&env_map)
    }

    /// Detect color profile from a specific set of environment variables.
    ///
    /// Useful for testing without modifying real environment.
    pub fn from_env(env: &[(&str, &str)]) -> Self {
        let map: HashMap<&str, &str> = env.iter().copied().collect();
        color_profile(true, &map)
    }
}

/// Check if NO_COLOR is set and considered truthy.
fn env_no_color(env: &HashMap<&str, &str>) -> bool {
    parse_bool(env.get("NO_COLOR").copied().unwrap_or(""))
}

/// Check if CLICOLOR_FORCE is set and considered truthy.
fn cli_color_forced(env: &HashMap<&str, &str>) -> bool {
    parse_bool(env.get("CLICOLOR_FORCE").copied().unwrap_or(""))
}

/// Check if CLICOLOR is set and considered truthy.
fn cli_color(env: &HashMap<&str, &str>) -> bool {
    parse_bool(env.get("CLICOLOR").copied().unwrap_or(""))
}

/// Check if COLORTERM indicates truecolor support.
fn color_term(env: &HashMap<&str, &str>) -> bool {
    match env.get("COLORTERM").map(|s| s.to_lowercase()) {
        Some(ct) => ct == "truecolor" || ct == "24bit" || ct == "yes" || ct == "true",
        None => false,
    }
}

/// Determine the color profile from environment variables.
///
/// This implements the same logic as charmbracelet/colorprofile's `colorProfile`
/// and `envColorProfile` functions.
fn color_profile(isatty: bool, env: &HashMap<&str, &str>) -> Profile {
    let term = env.get("TERM").copied();
    let term_str = term.unwrap_or("");
    let has_term = term.is_some();
    let is_dumb = !has_term || term_str == "dumb";

    let envp = env_color_profile(env);

    let mut p = if !isatty || is_dumb {
        Profile::NoTty
    } else {
        envp
    };

    // NO_COLOR takes precedence
    if env_no_color(env) && isatty {
        if p > Profile::Ascii {
            p = Profile::Ascii;
        }
        return p;
    }

    // CLICOLOR_FORCE
    if cli_color_forced(env) {
        if p < Profile::Ansi {
            p = Profile::Ansi;
        }
        if envp > p {
            p = envp;
        }
        return p;
    }

    // CLICOLOR
    if cli_color(env) && isatty && !is_dumb && p < Profile::Ansi {
        p = Profile::Ansi;
    }

    p
}

/// Infer the color profile purely from environment variable values
/// (TERM, COLORTERM, WT_SESSION, etc.), ignoring TTY checks.
fn env_color_profile(env: &HashMap<&str, &str>) -> Profile {
    let term = env.get("TERM").copied().unwrap_or("");
    let has_term = env.contains_key("TERM");

    let mut p = if !has_term || term.is_empty() || term == "dumb" {
        Profile::NoTty
    } else {
        Profile::Ansi
    };

    // Check for known TrueColor terminals
    let tc_terms = ["alacritty", "contour", "foot", "ghostty", "kitty", "rio", "wezterm"];
    for t in &tc_terms {
        if term.contains(t) {
            return Profile::TrueColor;
        }
    }

    // tmux / screen -> at least Ansi256
    if term.starts_with("tmux") || term.starts_with("screen") {
        if p < Profile::Ansi256 {
            p = Profile::Ansi256;
        }
    } else if term.starts_with("xterm")
        && p < Profile::Ansi {
            p = Profile::Ansi;
        }

    // Windows Terminal
    if env.get("WT_SESSION").is_some_and(|v| !v.is_empty()) {
        return Profile::TrueColor;
    }

    // Google Cloud Shell
    if parse_bool(env.get("GOOGLE_CLOUD_SHELL").copied().unwrap_or("")) {
        return Profile::TrueColor;
    }

    // COLORTERM (but not for screen/tmux which don't support it)
    if color_term(env) && !term.starts_with("screen") && !term.starts_with("tmux") {
        return Profile::TrueColor;
    }

    // 256color suffix
    if term.ends_with("256color") && p < Profile::Ansi256 {
        p = Profile::Ansi256;
    }

    // direct color suffix
    if term.ends_with("direct") {
        return Profile::TrueColor;
    }

    p
}

/// Parse a string as a boolean, mimicking Go's `strconv.ParseBool`.
/// "1", "t", "true", "yes", "y" (case insensitive) -> true
/// Everything else -> false
fn parse_bool(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "1" | "t" | "true" | "yes" | "y")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_from<'a>(pairs: &'a [(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
        pairs.to_vec()
    }

    #[test]
    fn test_empty_env() {
        // No TERM set -> NoTty
        let p = Profile::from_env(&env_from(&[]));
        assert_eq!(p, Profile::NoTty);
    }

    #[test]
    fn test_dumb_term() {
        let p = Profile::from_env(&env_from(&[("TERM", "dumb")]));
        assert_eq!(p, Profile::NoTty);
    }

    #[test]
    fn test_dumb_truecolor_not_forced() {
        let p = Profile::from_env(&env_from(&[("TERM", "dumb"), ("COLORTERM", "truecolor")]));
        assert_eq!(p, Profile::NoTty);
    }

    #[test]
    fn test_dumb_truecolor_forced() {
        let p = Profile::from_env(&env_from(&[("TERM", "dumb"), ("COLORTERM", "truecolor"), ("CLICOLOR_FORCE", "1")]));
        assert_eq!(p, Profile::TrueColor);
    }

    #[test]
    fn test_dumb_clicolor_force() {
        let p = Profile::from_env(&env_from(&[("TERM", "dumb"), ("CLICOLOR_FORCE", "1")]));
        assert_eq!(p, Profile::Ansi);
    }

    #[test]
    fn test_dumb_clicolor() {
        // CLICOLOR=1 with dumb term -> NoTty (dumb overrides)
        let p = Profile::from_env(&env_from(&[("TERM", "dumb"), ("CLICOLOR", "1")]));
        assert_eq!(p, Profile::NoTty);
    }

    #[test]
    fn test_xterm_256color() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm-256color")]));
        assert_eq!(p, Profile::Ansi256);
    }

    #[test]
    fn test_xterm_256color_clicolor() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm-256color"), ("CLICOLOR", "1")]));
        assert_eq!(p, Profile::Ansi256);
    }

    #[test]
    fn test_xterm_256color_colorterm_yes() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm-256color"), ("COLORTERM", "yes")]));
        assert_eq!(p, Profile::TrueColor);
    }

    #[test]
    fn test_xterm_256color_no_color() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm-256color"), ("NO_COLOR", "1")]));
        assert_eq!(p, Profile::Ascii);
    }

    #[test]
    fn test_xterm() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm")]));
        assert_eq!(p, Profile::Ansi);
    }

    #[test]
    fn test_xterm_no_color() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm"), ("NO_COLOR", "1")]));
        assert_eq!(p, Profile::Ascii);
    }

    #[test]
    fn test_xterm_clicolor() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm"), ("CLICOLOR", "1")]));
        assert_eq!(p, Profile::Ansi);
    }

    #[test]
    fn test_xterm_clicolor_force() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm"), ("CLICOLOR_FORCE", "1")]));
        assert_eq!(p, Profile::Ansi);
    }

    #[test]
    fn test_xterm_16color() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm-16color")]));
        assert_eq!(p, Profile::Ansi);
    }

    #[test]
    fn test_xterm_color() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm-color")]));
        assert_eq!(p, Profile::Ansi);
    }

    #[test]
    fn test_xterm_256color_no_color_clicolor_force() {
        // NO_COLOR takes precedence over CLICOLOR_FORCE
        let p = Profile::from_env(&env_from(&[("TERM", "xterm-256color"), ("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")]));
        assert_eq!(p, Profile::Ascii);
    }

    #[test]
    fn test_wt_session_with_xterm() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm-256color"), ("WT_SESSION", "1")]));
        assert_eq!(p, Profile::TrueColor);
    }

    #[test]
    fn test_screen() {
        let p = Profile::from_env(&env_from(&[("TERM", "screen")]));
        assert_eq!(p, Profile::Ansi256);
    }

    #[test]
    fn test_screen_colorterm() {
        // Screen doesn't support COLORTERM
        let p = Profile::from_env(&env_from(&[("TERM", "screen"), ("COLORTERM", "truecolor")]));
        assert_eq!(p, Profile::Ansi256);
    }

    #[test]
    fn test_tmux_colorterm() {
        // Tmux doesn't support COLORTERM
        let p = Profile::from_env(&env_from(&[("TERM", "tmux"), ("COLORTERM", "truecolor")]));
        assert_eq!(p, Profile::Ansi256);
    }

    #[test]
    fn test_tmux_256color() {
        let p = Profile::from_env(&env_from(&[("TERM", "tmux-256color")]));
        assert_eq!(p, Profile::Ansi256);
    }

    #[test]
    fn test_ignore_colorterm_no_term() {
        let p = Profile::from_env(&env_from(&[("COLORTERM", "truecolor")]));
        assert_eq!(p, Profile::NoTty);
    }

    #[test]
    fn test_xterm_direct() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm-direct")]));
        assert_eq!(p, Profile::TrueColor);
    }

    #[test]
    fn test_rio() {
        let p = Profile::from_env(&env_from(&[("TERM", "rio")]));
        assert_eq!(p, Profile::TrueColor);
    }

    #[test]
    fn test_kitty() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm-kitty")]));
        assert_eq!(p, Profile::TrueColor);
    }

    #[test]
    fn test_alacritty() {
        let p = Profile::from_env(&env_from(&[("TERM", "alacritty")]));
        assert_eq!(p, Profile::TrueColor);
    }

    #[test]
    fn test_ghostty() {
        let p = Profile::from_env(&env_from(&[("TERM", "xterm-ghostty")]));
        assert_eq!(p, Profile::TrueColor);
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("1"));
        assert!(parse_bool("true"));
        assert!(parse_bool("TRUE"));
        assert!(parse_bool("True"));
        assert!(parse_bool("yes"));
        assert!(parse_bool("y"));
        assert!(parse_bool("t"));
        assert!(!parse_bool("0"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool(""));
        assert!(!parse_bool("no"));
    }
}
