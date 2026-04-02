//! Kitty graphics protocol support for inline terminal images.

use crate::util::base64_encode;

/// Image format for Kitty graphics protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// RGBA 32-bit (format=32)
    Rgba,
    /// RGB 24-bit (format=24)
    Rgb,
    /// PNG compressed (format=100)
    Png,
}

/// Action for Kitty graphics protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KittyAction {
    /// Transmit image data
    Transmit,
    /// Transmit and display
    TransmitAndDisplay,
    /// Query terminal support
    Query,
    /// Display previously transmitted image
    Display,
    /// Delete image
    Delete,
}

/// Options for a Kitty graphics transmission.
#[derive(Debug, Clone)]
pub struct KittyOptions {
    pub action: KittyAction,
    pub format: ImageFormat,
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub compression: bool,
}

impl Default for KittyOptions {
    fn default() -> Self {
        Self {
            action: KittyAction::TransmitAndDisplay,
            format: ImageFormat::Rgba,
            id: 0,
            width: 0,
            height: 0,
            compression: false,
        }
    }
}

/// Encode image data as a Kitty graphics protocol sequence.
///
/// The data is base64-encoded and split into 4KB chunks.
pub fn encode_image(data: &[u8], opts: &KittyOptions) -> String {
    let b64 = base64_encode(data);
    let action = match opts.action {
        KittyAction::Transmit => "t",
        KittyAction::TransmitAndDisplay => "T",
        KittyAction::Query => "q",
        KittyAction::Display => "p",
        KittyAction::Delete => "d",
    };
    let format = match opts.format {
        ImageFormat::Rgba => 32,
        ImageFormat::Rgb => 24,
        ImageFormat::Png => 100,
    };

    let chunk_size = 4096;
    let mut result = String::new();
    let chunks: Vec<&str> = b64
        .as_bytes()
        .chunks(chunk_size)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    for (i, chunk) in chunks.iter().enumerate() {
        let more = if i < chunks.len() - 1 { 1 } else { 0 };
        if i == 0 {
            result.push_str(&format!(
                "\x1b_Ga={},f={},s={},v={},m={};{}\x1b\\",
                action, format, opts.width, opts.height, more, chunk
            ));
        } else {
            result.push_str(&format!("\x1b_Gm={};{}\x1b\\", more, chunk));
        }
    }

    result
}

/// Query Kitty graphics support.
pub fn query_support() -> &'static str {
    "\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_small_image() {
        let data = vec![255u8; 12]; // 1 RGBA pixel (4 bytes) x 3
        let opts = KittyOptions {
            width: 3,
            height: 1,
            ..Default::default()
        };
        let seq = encode_image(&data, &opts);
        assert!(seq.starts_with("\x1b_G"));
        assert!(seq.contains("a=T"));
        assert!(seq.contains("f=32"));
        assert!(seq.ends_with("\x1b\\"));
    }

    #[test]
    fn test_query_support() {
        let q = query_support();
        assert!(q.starts_with("\x1b_G"));
        assert!(q.contains("a=q"));
    }
}
