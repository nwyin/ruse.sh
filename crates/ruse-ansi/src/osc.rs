//! OSC (Operating System Command) sequence generation.

/// Set the window title (OSC 2).
pub fn set_window_title(title: &str) -> String {
    format!("\x1b]2;{}\x07", title)
}

/// Set the icon name (OSC 1).
pub fn set_icon_name(name: &str) -> String {
    format!("\x1b]1;{}\x07", name)
}

/// Set both icon name and window title (OSC 0).
pub fn set_icon_name_and_title(text: &str) -> String {
    format!("\x1b]0;{}\x07", text)
}

/// Open a hyperlink (OSC 8).
pub fn hyperlink_open(url: &str, params: &str) -> String {
    format!("\x1b]8;{};{}\x1b\\", params, url)
}

/// Close a hyperlink (OSC 8 with empty URL).
pub fn hyperlink_close() -> &'static str {
    "\x1b]8;;\x1b\\"
}

/// Set the system clipboard via OSC 52.
/// `selection`: `c` for clipboard, `p` for primary selection.
pub fn set_clipboard(content: &str, selection: char) -> String {
    let encoded = base64_encode(content.as_bytes());
    format!("\x1b]52;{};{}\x07", selection, encoded)
}

/// Request the system clipboard via OSC 52.
pub fn request_clipboard(selection: char) -> String {
    format!("\x1b]52;{};?\x07", selection)
}

/// Send a desktop notification (OSC 777).
pub fn notify(title: &str, body: &str) -> String {
    format!("\x1b]777;notify;{};{}\x07", title, body)
}

/// Set the working directory notification (OSC 7).
pub fn notify_working_directory(uri: &str) -> String {
    format!("\x1b]7;{}\x07", uri)
}

/// Simple base64 encoding.
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// Terminal query sequences.
pub mod query {
    /// Request cursor position report (DSR).
    pub fn cursor_position() -> &'static str {
        "\x1b[6n"
    }

    /// Request extended cursor position with page.
    pub fn extended_cursor_position() -> &'static str {
        "\x1b[?6n"
    }

    /// Request primary device attributes.
    pub fn device_attributes() -> &'static str {
        "\x1b[c"
    }

    /// Request secondary device attributes.
    pub fn device_attributes_secondary() -> &'static str {
        "\x1b[>c"
    }

    /// Request terminal version (XTVERSION).
    pub fn terminal_version() -> &'static str {
        "\x1b[>0q"
    }

    /// Request device status (DSR).
    pub fn device_status() -> &'static str {
        "\x1b[5n"
    }

    /// Request background color (OSC 11).
    pub fn background_color() -> &'static str {
        "\x1b]11;?\x07"
    }

    /// Request foreground color (OSC 10).
    pub fn foreground_color() -> &'static str {
        "\x1b]10;?\x07"
    }

    /// Request cursor color (OSC 12).
    pub fn cursor_color() -> &'static str {
        "\x1b]12;?\x07"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_title() {
        assert_eq!(set_window_title("Hello"), "\x1b]2;Hello\x07");
    }

    #[test]
    fn test_hyperlink() {
        let open = hyperlink_open("https://example.com", "");
        assert!(open.starts_with("\x1b]8;"));
        assert!(open.contains("https://example.com"));
    }

    #[test]
    fn test_clipboard_set() {
        let seq = set_clipboard("hello", 'c');
        assert!(seq.starts_with("\x1b]52;c;"));
        assert!(seq.ends_with("\x07"));
    }

    #[test]
    fn test_notify() {
        let n = notify("Title", "Body");
        assert!(n.contains("Title"));
        assert!(n.contains("Body"));
    }

    #[test]
    fn test_queries() {
        assert_eq!(query::cursor_position(), "\x1b[6n");
        assert_eq!(query::device_attributes(), "\x1b[c");
    }
}
