/// Hyperlink metadata (OSC 8).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Link {
    pub url: String,
    pub params: String,
}

impl Link {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            params: String::new(),
        }
    }

    pub fn with_params(url: impl Into<String>, params: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            params: params.into(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.url.is_empty() && self.params.is_empty()
    }

    pub fn reset(&mut self) {
        self.url.clear();
        self.params.clear();
    }

    /// Generate the OSC 8 open sequence: `\x1b]8;<params>;<url>\x1b\\`
    pub fn open_sequence(&self) -> String {
        format!("\x1b]8;{};{}\x1b\\", self.params, self.url)
    }

    /// Generate the OSC 8 close sequence: `\x1b]8;;\x1b\\`
    pub fn close_sequence() -> &'static str {
        "\x1b]8;;\x1b\\"
    }
}
