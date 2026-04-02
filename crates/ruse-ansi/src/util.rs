/// Simple base64 encoding.
pub(crate) fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
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

/// Convert ANSI 256-color index to RGB.
pub(crate) fn ansi_256_to_rgb(idx: u8) -> (u8, u8, u8) {
    if idx < 16 {
        ansi_basic_color(idx)
    } else if idx < 232 {
        let n = idx - 16;
        let b = (n % 6) * 51;
        let g = ((n / 6) % 6) * 51;
        let r = (n / 36) * 51;
        (r, g, b)
    } else {
        let v = 8 + (idx - 232) * 10;
        (v, v, v)
    }
}

/// Convert ANSI basic color index (0-15) to RGB.
pub(crate) fn ansi_basic_color(idx: u8) -> (u8, u8, u8) {
    match idx {
        0 => (0, 0, 0),
        1 => (170, 0, 0),
        2 => (0, 170, 0),
        3 => (170, 170, 0),
        4 => (0, 0, 170),
        5 => (170, 0, 170),
        6 => (0, 170, 170),
        7 => (170, 170, 170),
        8 => (85, 85, 85),
        9 => (255, 85, 85),
        10 => (85, 255, 85),
        11 => (255, 255, 85),
        12 => (85, 85, 255),
        13 => (255, 85, 255),
        14 => (85, 255, 255),
        15 => (255, 255, 255),
        _ => (0, 0, 0),
    }
}
