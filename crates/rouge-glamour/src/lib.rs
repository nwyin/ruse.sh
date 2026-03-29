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
