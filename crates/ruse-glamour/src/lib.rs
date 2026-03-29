pub mod renderer;
pub mod style;
pub mod themes;

pub use renderer::TermRenderer;
pub use style::StyleConfig;
pub use themes::get_theme;

/// Render markdown with a named theme and default word wrap at 80 columns.
pub fn render(markdown: &str, theme: &str) -> String {
    let style = get_theme(theme);
    TermRenderer::new(style).with_word_wrap(80).render(markdown)
}

/// Render markdown with the dark theme.
pub fn render_dark(markdown: &str) -> String {
    render(markdown, "dark")
}

/// Render markdown using the GLAMOUR_STYLE environment variable for theme
/// selection, falling back to "dark" if not set.
pub fn render_with_env(markdown: &str) -> String {
    let theme = std::env::var("GLAMOUR_STYLE").unwrap_or_else(|_| "dark".into());
    render(markdown, &theme)
}

/// Load a theme from a JSON file.
pub fn load_theme(path: &std::path::Path) -> Result<StyleConfig, Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(path)?;
    let config: StyleConfig = serde_json::from_str(&json)?;
    Ok(config)
}
