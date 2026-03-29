pub use ruse_colorprofile::Color;

/// Darken a color by multiplying its RGB components by `(1 - percent)`.
///
/// `percent` should be in `0.0..=1.0`. Returns `NoColor` unchanged.
pub fn darken(c: Color, percent: f64) -> Color {
    let Some((r, g, b)) = c.to_rgb() else {
        return c;
    };
    let factor = (1.0 - percent).clamp(0.0, 1.0);
    Color::Rgb {
        r: (r as f64 * factor).round() as u8,
        g: (g as f64 * factor).round() as u8,
        b: (b as f64 * factor).round() as u8,
    }
}

/// Lighten a color by adding `(255 * percent)` to each RGB component.
///
/// `percent` should be in `0.0..=1.0`. Returns `NoColor` unchanged.
pub fn lighten(c: Color, percent: f64) -> Color {
    let Some((r, g, b)) = c.to_rgb() else {
        return c;
    };
    let add = (255.0 * percent.clamp(0.0, 1.0)).round() as u16;
    Color::Rgb {
        r: (r as u16 + add).min(255) as u8,
        g: (g as u16 + add).min(255) as u8,
        b: (b as u16 + add).min(255) as u8,
    }
}

/// Return the complementary color (hue rotated 180 degrees).
///
/// Returns `NoColor` unchanged.
pub fn complementary(c: Color) -> Color {
    let Some((r, g, b)) = c.to_rgb() else {
        return c;
    };
    Color::Rgb {
        r: 255 - r,
        g: 255 - g,
        b: 255 - b,
    }
}

/// Return whether a color is "dark" (HSL luminance < 0.5).
///
/// Returns `true` for `NoColor` (treated as black).
pub fn is_dark(c: Color) -> bool {
    let Some((r, g, b)) = c.to_rgb() else {
        return true;
    };
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let luminance = (max + min) / 2.0;
    luminance < 0.5
}

/// Select a color based on terminal background brightness.
pub fn light_dark(is_dark_bg: bool, light: Color, dark: Color) -> Color {
    if is_dark_bg { dark } else { light }
}

/// Select a color variant based on color profile capability.
pub fn complete(
    profile: ruse_colorprofile::Profile,
    ansi: Color,
    ansi256: Color,
    truecolor: Color,
) -> Color {
    use ruse_colorprofile::Profile;
    match profile {
        Profile::TrueColor => truecolor,
        Profile::Ansi256 => ansi256,
        Profile::Ansi => ansi,
        _ => Color::NoColor,
    }
}

/// Generate a 2D gradient with rotation, returning a row-major array.
///
/// `width` and `height` define the grid size, `colors` are gradient stops,
/// `angle_deg` rotates the gradient (0 = left-to-right, 90 = top-to-bottom).
pub fn blend_2d(width: usize, height: usize, colors: &[Color], angle_deg: f64) -> Vec<Color> {
    if width == 0 || height == 0 || colors.is_empty() {
        return vec![];
    }

    let diagonal = ((width * width + height * height) as f64).sqrt();
    let gradient = blend_1d(diagonal.ceil() as usize + 1, colors);
    if gradient.is_empty() {
        return vec![Color::NoColor; width * height];
    }

    let angle_rad = angle_deg.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;

    let mut result = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let rot_x = dx * cos_a - dy * sin_a;
            // Map from [-diagonal/2, diagonal/2] to [0, gradient.len()-1]
            let t = (rot_x / diagonal + 0.5).clamp(0.0, 1.0);
            let idx = (t * (gradient.len() - 1) as f64).round() as usize;
            result.push(gradient[idx.min(gradient.len() - 1)]);
        }
    }
    result
}

/// Blend a series of color stops into `steps` evenly-distributed colors,
/// interpolating in CIELAB color space.
///
/// If `steps <= stops.len()`, returns the first `steps` stops.
/// If `stops` is empty, returns an empty vec.
/// NoColor stops are filtered out.
pub fn blend_1d(steps: usize, stops: &[Color]) -> Vec<Color> {
    use palette::{FromColor, Lab, Mix, Srgb};

    if steps == 0 {
        return vec![];
    }

    // Filter out NoColor
    let valid: Vec<Color> = stops.iter().copied().filter(|c| *c != Color::NoColor).collect();

    if valid.is_empty() {
        return vec![];
    }

    if valid.len() == 1 {
        return vec![valid[0]; steps];
    }

    if steps <= valid.len() {
        return valid[..steps].to_vec();
    }

    let to_lab = |c: Color| -> Lab {
        let (r, g, b) = c.to_rgb().unwrap();
        let srgb = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        Lab::from_color(srgb)
    };

    let from_lab = |lab: Lab| -> Color {
        let srgb: Srgb = Srgb::from_color(lab);
        Color::Rgb {
            r: (srgb.red.clamp(0.0, 1.0) * 255.0).round() as u8,
            g: (srgb.green.clamp(0.0, 1.0) * 255.0).round() as u8,
            b: (srgb.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
        }
    };

    let labs: Vec<Lab> = valid.iter().map(|c| to_lab(*c)).collect();

    let num_segments = labs.len() - 1;
    let default_size = steps / num_segments;
    let remaining = steps % num_segments;

    let mut result = Vec::with_capacity(steps);

    for i in 0..num_segments {
        let from = labs[i];
        let to = labs[i + 1];

        let segment_size = default_size + if i < remaining { 1 } else { 0 };

        let divisor = if segment_size > 1 { (segment_size - 1) as f32 } else { 1.0 };

        for j in 0..segment_size {
            let factor = if segment_size > 1 { j as f32 / divisor } else { 0.0 };
            let blended = from.mix(to, factor);
            result.push(from_lab(blended));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_darken() {
        let white = Color::Rgb { r: 255, g: 255, b: 255 };
        let darkened = darken(white, 0.5);
        assert_eq!(darkened, Color::Rgb { r: 128, g: 128, b: 128 });
    }

    #[test]
    fn test_lighten() {
        let black = Color::Rgb { r: 0, g: 0, b: 0 };
        let lightened = lighten(black, 0.5);
        assert_eq!(lightened, Color::Rgb { r: 128, g: 128, b: 128 });
    }

    #[test]
    fn test_complementary() {
        let red = Color::Rgb { r: 255, g: 0, b: 0 };
        assert_eq!(complementary(red), Color::Rgb { r: 0, g: 255, b: 255 });
    }

    #[test]
    fn test_is_dark() {
        assert!(is_dark(Color::Rgb { r: 0, g: 0, b: 0 }));
        assert!(!is_dark(Color::Rgb { r: 255, g: 255, b: 255 }));
        assert!(is_dark(Color::NoColor));
    }

    #[test]
    fn test_blend_1d_basic() {
        let black = Color::Rgb { r: 0, g: 0, b: 0 };
        let white = Color::Rgb { r: 255, g: 255, b: 255 };
        let result = blend_1d(3, &[black, white]);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], black);
        assert_eq!(result[2], white);
    }

    #[test]
    fn test_blend_1d_single_stop() {
        let red = Color::Rgb { r: 255, g: 0, b: 0 };
        let result = blend_1d(5, &[red]);
        assert_eq!(result.len(), 5);
        assert!(result.iter().all(|c| *c == red));
    }

    #[test]
    fn test_blend_1d_empty() {
        assert!(blend_1d(5, &[]).is_empty());
        assert!(blend_1d(0, &[Color::Rgb { r: 0, g: 0, b: 0 }]).is_empty());
    }

    #[test]
    fn test_nocolor_passthrough() {
        assert_eq!(darken(Color::NoColor, 0.5), Color::NoColor);
        assert_eq!(lighten(Color::NoColor, 0.5), Color::NoColor);
        assert_eq!(complementary(Color::NoColor), Color::NoColor);
    }
}
