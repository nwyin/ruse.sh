/// Encode RGB pixel data as a Sixel graphics sequence.
///
/// Pixels are in RGB format (3 bytes per pixel). Width and height specify
/// the image dimensions. The output is a DCS sequence that can be written
/// directly to a Sixel-capable terminal.
///
/// Color quantization reduces to at most 256 palette entries.
pub fn encode_sixel(pixels: &[u8], width: u32, height: u32) -> String {
    assert_eq!(
        pixels.len(),
        (width * height * 3) as usize,
        "pixel data must be width*height*3 bytes (RGB)"
    );

    let palette = build_palette(pixels);
    let indexed = quantize_pixels(pixels, &palette);

    let mut out = String::new();

    // DCS P1;P2;P3 q — start sixel data
    // P1=0 (pixel aspect ratio), P2=1 (background default), P3=0
    out.push_str("\x1bPq");

    // Emit palette definitions: #N;2;R;G;B (percentages 0-100)
    for (i, &(r, g, b)) in palette.iter().enumerate() {
        let rp = (r as u32 * 100) / 255;
        let gp = (g as u32 * 100) / 255;
        let bp = (b as u32 * 100) / 255;
        out.push_str(&format!("#{i};2;{rp};{gp};{bp}"));
    }

    // Sixel data: rows are processed in groups of 6 pixels tall
    let w = width as usize;
    let h = height as usize;

    for band_y in (0..h).step_by(6) {
        // For each color in this band
        for (color_idx, _) in palette.iter().enumerate() {
            let mut has_pixels = false;
            let mut row_data = Vec::with_capacity(w);

            for x in 0..w {
                let mut sixel_bits: u8 = 0;
                for dy in 0..6 {
                    let y = band_y + dy;
                    if y < h {
                        let pixel_idx = y * w + x;
                        if indexed[pixel_idx] == color_idx as u8 {
                            sixel_bits |= 1 << dy;
                            has_pixels = true;
                        }
                    }
                }
                row_data.push(sixel_bits);
            }

            if has_pixels {
                out.push('#');
                out.push_str(&color_idx.to_string());

                // RLE-encode the sixel data
                let mut i = 0;
                while i < row_data.len() {
                    let val = row_data[i];
                    let mut count = 1;
                    while i + count < row_data.len() && row_data[i + count] == val {
                        count += 1;
                    }
                    let ch = (val + 63) as char;
                    if count > 3 {
                        out.push('!');
                        out.push_str(&count.to_string());
                        out.push(ch);
                    } else {
                        for _ in 0..count {
                            out.push(ch);
                        }
                    }
                    i += count;
                }

                out.push('$'); // carriage return (same band)
            }
        }
        out.push('-'); // newline (next band)
    }

    // ST — end sixel data
    out.push_str("\x1b\\");
    out
}

/// Build a color palette from RGB pixel data (at most 256 colors).
/// Uses simple hash-based deduplication; for images with >256 unique colors,
/// only the first 256 found are kept.
fn build_palette(pixels: &[u8]) -> Vec<(u8, u8, u8)> {
    let mut palette = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for chunk in pixels.chunks(3) {
        if chunk.len() < 3 {
            break;
        }
        let color = (chunk[0], chunk[1], chunk[2]);
        if seen.insert(color) {
            palette.push(color);
            if palette.len() >= 256 {
                break;
            }
        }
    }

    if palette.is_empty() {
        palette.push((0, 0, 0));
    }

    palette
}

/// Map each pixel to the nearest palette index.
fn quantize_pixels(pixels: &[u8], palette: &[(u8, u8, u8)]) -> Vec<u8> {
    let mut indexed = Vec::with_capacity(pixels.len() / 3);
    for chunk in pixels.chunks(3) {
        if chunk.len() < 3 {
            break;
        }
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];

        let mut best = 0u8;
        let mut best_dist = u32::MAX;
        for (i, &(pr, pg, pb)) in palette.iter().enumerate() {
            let dr = (r as i32 - pr as i32).unsigned_abs();
            let dg = (g as i32 - pg as i32).unsigned_abs();
            let db = (b as i32 - pb as i32).unsigned_abs();
            let dist = dr * dr + dg * dg + db * db;
            if dist < best_dist {
                best_dist = dist;
                best = i as u8;
                if dist == 0 {
                    break;
                }
            }
        }
        indexed.push(best);
    }
    indexed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_solid_red() {
        // 2x6 solid red image
        let pixels = [255u8, 0, 0].repeat(2 * 6);
        let result = encode_sixel(&pixels, 2, 6);
        assert!(result.starts_with("\x1bPq"));
        assert!(result.ends_with("\x1b\\"));
        // Should have palette entry for red
        assert!(result.contains("#0;2;100;0;0"));
    }

    #[test]
    fn test_encode_two_colors() {
        // 2x1 image: one red pixel, one blue pixel
        let pixels = vec![255, 0, 0, 0, 0, 255];
        let result = encode_sixel(&pixels, 2, 1);
        assert!(result.starts_with("\x1bPq"));
        // Should have two palette entries
        assert!(result.contains("#0;2;"));
        assert!(result.contains("#1;2;"));
    }

    #[test]
    fn test_build_palette_dedup() {
        let pixels = vec![255, 0, 0, 255, 0, 0, 0, 255, 0];
        let palette = build_palette(&pixels);
        assert_eq!(palette.len(), 2);
    }

    #[test]
    fn test_quantize_exact() {
        let palette = vec![(255, 0, 0), (0, 255, 0)];
        let pixels = vec![255, 0, 0, 0, 255, 0, 255, 0, 0];
        let indexed = quantize_pixels(&pixels, &palette);
        assert_eq!(indexed, vec![0, 1, 0]);
    }
}
