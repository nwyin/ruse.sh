use std::collections::HashMap;
use std::sync::RwLock;

use crate::color::{ANSI256_TO_16, Color};
use crate::profile::Profile;

/// 6x6x6 color cube thresholds: 0, 95, 135, 175, 215, 255
const Q2C: [i32; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];

/// Thread-safe cache for color conversions to avoid repeated computation.
static CACHE_256: std::sync::LazyLock<RwLock<HashMap<Color, Color>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));
static CACHE_16: std::sync::LazyLock<RwLock<HashMap<Color, Color>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

impl Profile {
    /// Convert/downsample a color to match this profile's capability.
    ///
    /// Results are cached for `Ansi256` and `Ansi` profiles to avoid
    /// repeated computation of the same conversions.
    pub fn convert(&self, color: Color) -> Color {
        match self {
            Profile::TrueColor => color,
            Profile::Ansi256 => cached_convert(&CACHE_256, color, convert_to_256),
            Profile::Ansi => cached_convert(&CACHE_16, color, convert_to_16),
            Profile::Ascii | Profile::NoTty => Color::NoColor,
        }
    }
}

/// Look up a color conversion in cache, or compute and cache it.
fn cached_convert(
    cache: &RwLock<HashMap<Color, Color>>,
    color: Color,
    convert_fn: fn(Color) -> Color,
) -> Color {
    // Fast path: read lock
    if let Ok(guard) = cache.read() {
        if let Some(cached) = guard.get(&color) {
            return *cached;
        }
    }

    // Slow path: compute and cache
    let result = convert_fn(color);
    if let Ok(mut guard) = cache.write() {
        guard.entry(color).or_insert(result);
    }
    result
}

/// Map an 8-bit value to the nearest 6-cube index (0-5).
/// Ported from charmbracelet/x/ansi `to6Cube`.
fn to_6cube(v: i32) -> usize {
    if v < 48 {
        0
    } else if v < 115 {
        1
    } else {
        ((v - 35) / 40) as usize
    }
}

/// Squared Euclidean distance between two RGB triples.
fn dist_sq(r1: i32, g1: i32, b1: i32, r2: i32, g2: i32, b2: i32) -> i32 {
    (r1 - r2) * (r1 - r2) + (g1 - g2) * (g1 - g2) + (b1 - b2) * (b1 - b2)
}

/// Convert an arbitrary color to the nearest ANSI 256-color index.
fn convert_to_256(color: Color) -> Color {
    match color {
        Color::NoColor => Color::NoColor,
        Color::Basic(n) => Color::Basic(n),
        Color::Indexed(n) => Color::Indexed(n),
        Color::Rgb { r, g, b } => {
            let ri = r as i32;
            let gi = g as i32;
            let bi = b as i32;

            // Map RGB to 6x6x6 cube indices
            let qr = to_6cube(ri);
            let cr = Q2C[qr];
            let qg = to_6cube(gi);
            let cg = Q2C[qg];
            let qb = to_6cube(bi);
            let cb = Q2C[qb];

            // Cube color index
            let ci = 36 * qr + 6 * qg + qb;

            // If exact match, return cube color immediately
            if cr == ri && cg == gi && cb == bi {
                return Color::Indexed((16 + ci) as u8);
            }

            // Work out the closest grey (average of RGB)
            let grey_avg = (ri + gi + bi) / 3;
            let grey_idx = if grey_avg > 238 { 23 } else { (grey_avg - 3).max(0) / 10 };
            let grey = 8 + 10 * grey_idx;

            // Use Euclidean distance in RGB to choose between cube and greyscale.
            let color_dist = dist_sq(cr, cg, cb, ri, gi, bi);
            let grey_dist = dist_sq(grey, grey, grey, ri, gi, bi);

            if color_dist <= grey_dist {
                Color::Indexed((16 + ci) as u8)
            } else {
                Color::Indexed((232 + grey_idx) as u8)
            }
        }
    }
}

/// Convert an arbitrary color to the nearest ANSI 16-color (basic) color.
fn convert_to_16(color: Color) -> Color {
    match color {
        Color::NoColor => Color::NoColor,
        Color::Basic(n) => Color::Basic(n),
        Color::Indexed(n) => Color::Basic(ANSI256_TO_16[n as usize]),
        Color::Rgb { r, g, b } => {
            // First convert to 256, then map to 16
            let c256 = convert_to_256(Color::Rgb { r, g, b });
            match c256 {
                Color::Indexed(n) => Color::Basic(ANSI256_TO_16[n as usize]),
                _ => Color::NoColor,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truecolor_passthrough() {
        let c = Color::Rgb { r: 255, g: 128, b: 0 };
        assert_eq!(Profile::TrueColor.convert(c), c);
    }

    #[test]
    fn test_ansi256_basic_passthrough() {
        let c = Color::Basic(1);
        assert_eq!(Profile::Ansi256.convert(c), c);
    }

    #[test]
    fn test_ansi256_indexed_passthrough() {
        let c = Color::Indexed(196);
        assert_eq!(Profile::Ansi256.convert(c), c);
    }

    #[test]
    fn test_ansi256_red() {
        // Pure red -> index 196
        let c = Color::Rgb { r: 255, g: 0, b: 0 };
        assert_eq!(Profile::Ansi256.convert(c), Color::Indexed(196));
    }

    #[test]
    fn test_ansi256_white() {
        let c = Color::Rgb { r: 255, g: 255, b: 255 };
        assert_eq!(Profile::Ansi256.convert(c), Color::Indexed(231));
    }

    #[test]
    fn test_ansi256_gray() {
        // 0x80 = 128 => grey average = 128, grey_idx = (128-3)/10 = 12, grey = 128
        // cube: to_6cube(128) = (128-35)/40 = 2 => Q2C[2] = 0x87 = 135
        // cube color = (135,135,135) dist from (128,128,128) = 3*49 = 147
        // grey 128 is exact match dist = 0
        let c = Color::Rgb { r: 128, g: 128, b: 128 };
        assert_eq!(Profile::Ansi256.convert(c), Color::Indexed(244));
    }

    #[test]
    fn test_ansi256_offwhite() {
        // #eeeeee = (238,238,238) => grey_avg=238, grey_idx=(238-3)/10=23, grey=238
        // Exact grey match => 232+23 = 255
        let c = Color::Rgb { r: 0xee, g: 0xee, b: 0xee };
        assert_eq!(Profile::Ansi256.convert(c), Color::Indexed(255));
    }

    #[test]
    fn test_ansi256_ff8537() {
        // #ff8537 = (255, 133, 55)
        // qr = to_6cube(255) = (255-35)/40 = 5, cr = 0xff = 255
        // qg = to_6cube(133) = (133-35)/40 = 2, cg = 0x87 = 135
        // qb = to_6cube(55) = 1, cb = 0x5f = 95
        // ci = 36*5 + 6*2 + 1 = 193
        // Index = 16 + 193 = 209
        let c = Color::Rgb { r: 255, g: 133, b: 55 };
        assert_eq!(Profile::Ansi256.convert(c), Color::Indexed(209));
    }

    #[test]
    fn test_ansi_from_indexed_196() {
        // 196 -> ANSI256_TO_16[196] = 9
        let c = Color::Indexed(196);
        assert_eq!(Profile::Ansi.convert(c), Color::Basic(9));
    }

    #[test]
    fn test_ansi_from_rgb() {
        // #ff8537 -> 256=209 -> 16=9 (bright red)
        let c = Color::Rgb { r: 255, g: 133, b: 55 };
        assert_eq!(Profile::Ansi.convert(c), Color::Basic(9));
    }

    #[test]
    fn test_ansi_basic_passthrough() {
        let c = Color::Basic(3);
        assert_eq!(Profile::Ansi.convert(c), Color::Basic(3));
    }

    #[test]
    fn test_ascii_returns_nocolor() {
        let c = Color::Rgb { r: 255, g: 0, b: 0 };
        assert_eq!(Profile::Ascii.convert(c), Color::NoColor);
    }

    #[test]
    fn test_notty_returns_nocolor() {
        let c = Color::Basic(5);
        assert_eq!(Profile::NoTty.convert(c), Color::NoColor);
    }

    #[test]
    fn test_ansi256_silver_foil() {
        // #afafaf = (175, 175, 175) => cube match is exact at Q2C[3]=175
        // ci = 36*3 + 6*3 + 3 = 129, index = 16 + 129 = 145
        let c = Color::Rgb { r: 0xaf, g: 0xaf, b: 0xaf };
        assert_eq!(Profile::Ansi256.convert(c), Color::Indexed(145));
    }

    #[test]
    fn test_ansi_white() {
        // RGB (255,255,255) -> 256=231 -> 16=15 (bright white)
        let c = Color::Rgb { r: 255, g: 255, b: 255 };
        assert_eq!(Profile::Ansi.convert(c), Color::Basic(15));
    }
}
