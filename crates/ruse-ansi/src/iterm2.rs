use crate::util::base64_encode;

/// Encode image data as an iTerm2 inline image (OSC 1337).
///
/// The data should be the raw file content (PNG, JPEG, etc.).
/// Width/height are in cells; `None` means auto-size.
pub fn encode_iterm2_image(
    data: &[u8],
    width: Option<u32>,
    height: Option<u32>,
    preserve_aspect: bool,
) -> String {
    let encoded = base64_encode(data);

    let mut params = String::from("inline=1");
    if let Some(w) = width {
        params.push_str(&format!(";width={w}"));
    }
    if let Some(h) = height {
        params.push_str(&format!(";height={h}"));
    }
    if preserve_aspect {
        params.push_str(";preserveAspectRatio=1");
    }
    params.push_str(&format!(";size={}", data.len()));

    format!("\x1b]1337;File={params}:{encoded}\x07")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_basic() {
        let data = b"\x89PNG"; // Fake PNG header
        let result = encode_iterm2_image(data, None, None, true);
        assert!(result.starts_with("\x1b]1337;File="));
        assert!(result.contains("inline=1"));
        assert!(result.contains("preserveAspectRatio=1"));
        assert!(result.ends_with("\x07"));
    }

    #[test]
    fn test_encode_with_dimensions() {
        let data = b"test";
        let result = encode_iterm2_image(data, Some(40), Some(20), false);
        assert!(result.contains("width=40"));
        assert!(result.contains("height=20"));
        assert!(!result.contains("preserveAspectRatio"));
    }
}
