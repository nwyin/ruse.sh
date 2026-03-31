use crate::SgrStyle;

/// Render RGB pixel data as Unicode half-block characters.
///
/// Each terminal cell represents 2 vertical pixels using the lower half block
/// character (▄) with foreground = bottom pixel, background = top pixel.
/// This works in any terminal with Unicode and TrueColor support.
///
/// Pixels are in RGB format (3 bytes per pixel).
pub fn render_mosaic(pixels: &[u8], width: u32, height: u32) -> String {
    assert_eq!(pixels.len(), (width * height * 3) as usize, "pixel data must be width*height*3 bytes (RGB)");

    let w = width as usize;
    let h = height as usize;
    let mut out = String::new();

    // Process rows in pairs (top, bottom)
    let mut y = 0;
    while y < h {
        if y > 0 {
            out.push('\n');
        }

        for x in 0..w {
            let top_idx = (y * w + x) * 3;
            let (tr, tg, tb) = (pixels[top_idx], pixels[top_idx + 1], pixels[top_idx + 2]);

            if y + 1 < h {
                // Two rows: top = bg, bottom = fg, char = ▄
                let bot_idx = ((y + 1) * w + x) * 3;
                let (br, bg, bb) = (pixels[bot_idx], pixels[bot_idx + 1], pixels[bot_idx + 2]);

                let sgr = SgrStyle::new()
                    .fg_rgb(br, bg, bb)
                    .bg_rgb(tr, tg, tb);
                out.push_str(&sgr.styled("\u{2584}")); // ▄
            } else {
                // Odd last row: top only, use upper half block with fg
                let sgr = SgrStyle::new().fg_rgb(tr, tg, tb);
                out.push_str(&sgr.styled("\u{2580}")); // ▀
            }
        }

        y += 2;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strip_ansi;

    #[test]
    fn test_render_2x2() {
        // 2x2 image: all red
        let pixels = [255u8, 0, 0].repeat(2 * 2);
        let result = render_mosaic(&pixels, 2, 2);
        let plain = strip_ansi(&result);
        // Should produce 1 row of 2 half-block chars
        assert_eq!(plain.chars().count(), 2);
        assert!(plain.contains('\u{2584}'));
    }

    #[test]
    fn test_render_4x4() {
        let pixels = [0u8, 128, 255].repeat(4 * 4);
        let result = render_mosaic(&pixels, 4, 4);
        let lines: Vec<&str> = result.split('\n').collect();
        // 4 pixel rows -> 2 terminal rows
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_render_odd_height() {
        // 2x3 image -> 2 terminal rows (first pair + odd last row)
        let pixels = [128u8, 128, 128].repeat(2 * 3);
        let result = render_mosaic(&pixels, 2, 3);
        let lines: Vec<&str> = result.split('\n').collect();
        assert_eq!(lines.len(), 2);
        let plain = strip_ansi(&result);
        // Last row should use ▀ (upper half block)
        assert!(plain.contains('\u{2580}'));
    }
}
