//! 256-color palette management.

/// A 256-entry terminal color palette.
#[derive(Clone)]
pub struct Palette {
    colors: [(u8, u8, u8); 256],
}

impl Palette {
    /// Create the standard XTerm 256-color palette.
    pub fn xterm() -> Self {
        let mut colors = [(0u8, 0u8, 0u8); 256];

        // Standard colors (0-7)
        colors[0] = (0, 0, 0);
        colors[1] = (128, 0, 0);
        colors[2] = (0, 128, 0);
        colors[3] = (128, 128, 0);
        colors[4] = (0, 0, 128);
        colors[5] = (128, 0, 128);
        colors[6] = (0, 128, 128);
        colors[7] = (192, 192, 192);

        // Bright colors (8-15)
        colors[8] = (128, 128, 128);
        colors[9] = (255, 0, 0);
        colors[10] = (0, 255, 0);
        colors[11] = (255, 255, 0);
        colors[12] = (0, 0, 255);
        colors[13] = (255, 0, 255);
        colors[14] = (0, 255, 255);
        colors[15] = (255, 255, 255);

        // 6x6x6 color cube (16-231)
        for r in 0..6u8 {
            for g in 0..6u8 {
                for b in 0..6u8 {
                    let idx = 16 + (r as usize * 36) + (g as usize * 6) + b as usize;
                    let rv = if r == 0 { 0 } else { 55 + r * 40 };
                    let gv = if g == 0 { 0 } else { 55 + g * 40 };
                    let bv = if b == 0 { 0 } else { 55 + b * 40 };
                    colors[idx] = (rv, gv, bv);
                }
            }
        }

        // Grayscale ramp (232-255)
        for i in 0..24u8 {
            let v = 8 + i * 10;
            colors[232 + i as usize] = (v, v, v);
        }

        Self { colors }
    }

    /// Get the RGB color for a palette index.
    pub fn get(&self, idx: u8) -> (u8, u8, u8) {
        self.colors[idx as usize]
    }

    /// Set a custom color at a palette index.
    pub fn set(&mut self, idx: u8, r: u8, g: u8, b: u8) {
        self.colors[idx as usize] = (r, g, b);
    }

    /// Generate OSC 4 sequence to set a palette color.
    pub fn set_color_sequence(idx: u8, r: u8, g: u8, b: u8) -> String {
        format!("\x1b]4;{};rgb:{:02x}/{:02x}/{:02x}\x07", idx, r, g, b)
    }

    /// Generate OSC 4 sequence to query a palette color.
    pub fn query_color_sequence(idx: u8) -> String {
        format!("\x1b]4;{};?\x07", idx)
    }

    /// Generate OSC 104 sequence to reset a palette color to default.
    pub fn reset_color_sequence(idx: u8) -> String {
        format!("\x1b]104;{}\x07", idx)
    }

    /// Reset all palette colors.
    pub fn reset_all_sequence() -> &'static str {
        "\x1b]104\x07"
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::xterm()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xterm_basic_colors() {
        let p = Palette::xterm();
        assert_eq!(p.get(0), (0, 0, 0)); // black
        assert_eq!(p.get(1), (128, 0, 0)); // red
        assert_eq!(p.get(15), (255, 255, 255)); // white
    }

    #[test]
    fn test_color_cube() {
        let p = Palette::xterm();
        // Index 16 = (0,0,0) in cube
        assert_eq!(p.get(16), (0, 0, 0));
        // Index 196 = (5,0,0) = (255, 0, 0)
        assert_eq!(p.get(196), (255, 0, 0));
    }

    #[test]
    fn test_grayscale() {
        let p = Palette::xterm();
        assert_eq!(p.get(232), (8, 8, 8)); // darkest
        assert_eq!(p.get(255), (238, 238, 238)); // lightest
    }

    #[test]
    fn test_custom_color() {
        let mut p = Palette::xterm();
        p.set(0, 10, 20, 30);
        assert_eq!(p.get(0), (10, 20, 30));
    }

    #[test]
    fn test_sequences() {
        let seq = Palette::set_color_sequence(1, 255, 0, 0);
        assert!(seq.contains("4;1;"));
        assert!(seq.contains("rgb:ff/00/00"));
    }
}
