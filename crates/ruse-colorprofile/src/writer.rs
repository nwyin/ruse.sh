use std::io::{self, Write};

use ruse_ansi::strip_ansi;

use crate::color::Color;
use crate::profile::Profile;

/// A writer that intercepts ANSI SGR sequences and downgrades colors
/// to match the terminal's detected profile.
pub struct ProfileWriter<W: Write> {
    inner: W,
    profile: Profile,
}

impl<W: Write> ProfileWriter<W> {
    pub fn new(inner: W, profile: Profile) -> Self {
        Self { inner, profile }
    }

    /// Get a reference to the profile.
    pub fn profile(&self) -> Profile {
        self.profile
    }

    /// Get a mutable reference to the inner writer.
    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    /// Consume the ProfileWriter and return the inner writer.
    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for ProfileWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.profile {
            Profile::TrueColor => {
                self.inner.write_all(buf)?;
                Ok(buf.len())
            }
            Profile::NoTty => {
                let s = String::from_utf8_lossy(buf);
                let stripped = strip_ansi(&s);
                self.inner.write_all(stripped.as_bytes())?;
                Ok(buf.len())
            }
            Profile::Ascii | Profile::Ansi | Profile::Ansi256 => {
                let output = downsample(buf, self.profile);
                self.inner.write_all(&output)?;
                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// State machine for parsing ANSI escape sequences and downsampling colors.
fn downsample(input: &[u8], profile: Profile) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;

    while i < input.len() {
        if input[i] == 0x1b && i + 1 < input.len() && input[i + 1] == b'[' {
            // Start of CSI sequence
            let start = i;
            i += 2; // skip ESC[

            // Collect parameter bytes (0x30-0x3F) and intermediate bytes (0x20-0x2F)
            let params_start = i;
            while i < input.len() && (0x20..=0x3f).contains(&input[i]) {
                i += 1;
            }
            let params_end = i;

            // Final byte (0x40-0x7E)
            if i < input.len() && (0x40..=0x7e).contains(&input[i]) {
                let final_byte = input[i];
                i += 1;

                if final_byte == b'm' {
                    // This is an SGR sequence - process it
                    let params_bytes = &input[params_start..params_end];
                    let sgr = handle_sgr(params_bytes, profile);
                    out.extend_from_slice(sgr.as_bytes());
                } else {
                    // Non-SGR CSI sequence - pass through
                    out.extend_from_slice(&input[start..i]);
                }
            } else {
                // Incomplete/invalid CSI sequence - pass through
                out.extend_from_slice(&input[start..i]);
            }
        } else {
            out.push(input[i]);
            i += 1;
        }
    }

    out
}

/// A parsed SGR parameter that may use either ';' or ':' as separator.
#[derive(Debug, Clone)]
struct SgrParam {
    value: i32,
    /// Was this parameter separated from the previous by ':' (subparameter)?
    is_sub: bool,
}

/// Parse SGR parameter bytes into a list of parameters, tracking separator type.
fn parse_sgr_params(params: &[u8]) -> Vec<SgrParam> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut is_sub = false;

    for &b in params {
        if b == b';' {
            let value = if current.is_empty() {
                0
            } else {
                current.parse::<i32>().unwrap_or(0)
            };
            result.push(SgrParam { value, is_sub });
            current.clear();
            is_sub = false;
        } else if b == b':' {
            let value = if current.is_empty() {
                0
            } else {
                current.parse::<i32>().unwrap_or(0)
            };
            result.push(SgrParam { value, is_sub });
            current.clear();
            is_sub = true;
        } else {
            current.push(b as char);
        }
    }
    // Last parameter
    let value = if current.is_empty() {
        0
    } else {
        current.parse::<i32>().unwrap_or(0)
    };
    result.push(SgrParam { value, is_sub });

    result
}

/// Process an SGR sequence and return the downsampled replacement string.
fn handle_sgr(params_bytes: &[u8], profile: Profile) -> String {
    let params = parse_sgr_params(params_bytes);
    let mut style_parts: Vec<String> = Vec::new();
    let mut i = 0;

    while i < params.len() {
        let p = params[i].value;

        match p {
            0 => {
                // SGR reset - use empty string to emit "\x1b[m"
                style_parts.push(String::new());
                i += 1;
            }
            // Basic foreground colors 30-37
            30..=37 => {
                if profile >= Profile::Ansi {
                    let basic = (p - 30) as u8;
                    let converted = profile.convert(Color::Basic(basic));
                    push_fg_color(&mut style_parts, converted);
                }
                i += 1;
            }
            // Extended foreground color
            38 => {
                let consumed = handle_extended_color(&params, i, profile, true, &mut style_parts);
                i += consumed;
            }
            // Default foreground
            39 => {
                if profile >= Profile::Ansi {
                    style_parts.push("39".to_string());
                }
                i += 1;
            }
            // Basic background colors 40-47
            40..=47 => {
                if profile >= Profile::Ansi {
                    let basic = (p - 40) as u8;
                    let converted = profile.convert(Color::Basic(basic));
                    push_bg_color(&mut style_parts, converted);
                }
                i += 1;
            }
            // Extended background color
            48 => {
                let consumed = handle_extended_color(&params, i, profile, false, &mut style_parts);
                i += consumed;
            }
            // Default background
            49 => {
                if profile >= Profile::Ansi {
                    style_parts.push("49".to_string());
                }
                i += 1;
            }
            // Extended underline color
            58 => {
                let consumed = handle_extended_color_ul(&params, i, profile, &mut style_parts);
                i += consumed;
            }
            // Default underline color
            59 => {
                if profile >= Profile::Ansi {
                    style_parts.push("59".to_string());
                }
                i += 1;
            }
            // Bright foreground colors 90-97
            90..=97 => {
                if profile >= Profile::Ansi {
                    let basic = (p - 90 + 8) as u8;
                    let converted = profile.convert(Color::Basic(basic));
                    push_fg_color(&mut style_parts, converted);
                }
                i += 1;
            }
            // Bright background colors 100-107
            100..=107 => {
                if profile >= Profile::Ansi {
                    let basic = (p - 100 + 8) as u8;
                    let converted = profile.convert(Color::Basic(basic));
                    push_bg_color(&mut style_parts, converted);
                }
                i += 1;
            }
            // All other attributes (bold, italic, etc.) - pass through
            _ => {
                style_parts.push(p.to_string());
                i += 1;
            }
        }
    }

    format!("\x1b[{}m", style_parts.join(";"))
}

/// Handle extended color sequences (38;5;N, 38;2;R;G;B, 38:5:N, 38:2::R:G:B, etc.)
/// Returns the number of params consumed.
fn handle_extended_color(
    params: &[SgrParam],
    start: usize,
    profile: Profile,
    is_fg: bool,
    style_parts: &mut Vec<String>,
) -> usize {
    if start + 1 >= params.len() {
        return 1;
    }

    let color_type = params[start + 1].value;
    let uses_colon = params[start + 1].is_sub;

    match color_type {
        5 => {
            // Indexed color: 38;5;N or 38:5:N
            if start + 2 >= params.len() {
                return 2;
            }
            let idx = params[start + 2].value as u8;
            let color = if idx < 16 {
                Color::Basic(idx)
            } else {
                Color::Indexed(idx)
            };

            if profile >= Profile::Ansi {
                let converted = profile.convert(color);
                if is_fg {
                    push_fg_color(style_parts, converted);
                } else {
                    push_bg_color(style_parts, converted);
                }
            }
            3
        }
        2 => {
            // RGB color
            if uses_colon {
                // Colon-separated: 38:2::R:G:B (with optional color space after 2)
                // Find R, G, B - they follow after possible empty color space param
                let mut offset = start + 2;
                // Skip optional empty colorspace parameter
                if offset < params.len() && params[offset].is_sub {
                    offset += 1;
                }
                if offset + 2 >= params.len() {
                    return params.len() - start;
                }
                let r = params[offset].value as u8;
                let g = params[offset + 1].value as u8;
                let b = params[offset + 2].value as u8;
                let color = Color::Rgb { r, g, b };

                if profile >= Profile::Ansi {
                    let converted = profile.convert(color);
                    if is_fg {
                        push_fg_color(style_parts, converted);
                    } else {
                        push_bg_color(style_parts, converted);
                    }
                }
                offset + 3 - start
            } else {
                // Semicolon-separated: 38;2;R;G;B
                if start + 4 >= params.len() {
                    return params.len() - start;
                }
                let r = params[start + 2].value as u8;
                let g = params[start + 3].value as u8;
                let b = params[start + 4].value as u8;
                let color = Color::Rgb { r, g, b };

                if profile >= Profile::Ansi {
                    let converted = profile.convert(color);
                    if is_fg {
                        push_fg_color(style_parts, converted);
                    } else {
                        push_bg_color(style_parts, converted);
                    }
                }
                5
            }
        }
        _ => 2,
    }
}

/// Handle extended underline color sequences (58;5;N, 58;2;R;G;B, etc.)
fn handle_extended_color_ul(
    params: &[SgrParam],
    start: usize,
    profile: Profile,
    style_parts: &mut Vec<String>,
) -> usize {
    if start + 1 >= params.len() {
        return 1;
    }

    let color_type = params[start + 1].value;
    let uses_colon = params[start + 1].is_sub;

    match color_type {
        5 => {
            if start + 2 >= params.len() {
                return 2;
            }
            let idx = params[start + 2].value as u8;
            let color = if idx < 16 {
                Color::Basic(idx)
            } else {
                Color::Indexed(idx)
            };

            if profile >= Profile::Ansi {
                let converted = profile.convert(color);
                push_ul_color(style_parts, converted);
            }
            3
        }
        2 => {
            if uses_colon {
                let mut offset = start + 2;
                if offset < params.len() && params[offset].is_sub {
                    offset += 1;
                }
                if offset + 2 >= params.len() {
                    return params.len() - start;
                }
                let r = params[offset].value as u8;
                let g = params[offset + 1].value as u8;
                let b = params[offset + 2].value as u8;
                let color = Color::Rgb { r, g, b };
                if profile >= Profile::Ansi {
                    let converted = profile.convert(color);
                    push_ul_color(style_parts, converted);
                }
                offset + 3 - start
            } else {
                if start + 4 >= params.len() {
                    return params.len() - start;
                }
                let r = params[start + 2].value as u8;
                let g = params[start + 3].value as u8;
                let b = params[start + 4].value as u8;
                let color = Color::Rgb { r, g, b };
                if profile >= Profile::Ansi {
                    let converted = profile.convert(color);
                    push_ul_color(style_parts, converted);
                }
                5
            }
        }
        _ => 2,
    }
}

/// Push a foreground color into the style parts.
fn push_fg_color(parts: &mut Vec<String>, color: Color) {
    match color {
        Color::NoColor => {}
        Color::Basic(n) if n < 8 => parts.push((30 + n as u32).to_string()),
        Color::Basic(n) => parts.push((90 + (n - 8) as u32).to_string()),
        Color::Indexed(n) => parts.push(format!("38;5;{n}")),
        Color::Rgb { r, g, b } => parts.push(format!("38;2;{r};{g};{b}")),
    }
}

/// Push a background color into the style parts.
fn push_bg_color(parts: &mut Vec<String>, color: Color) {
    match color {
        Color::NoColor => {}
        Color::Basic(n) if n < 8 => parts.push((40 + n as u32).to_string()),
        Color::Basic(n) => parts.push((100 + (n - 8) as u32).to_string()),
        Color::Indexed(n) => parts.push(format!("48;5;{n}")),
        Color::Rgb { r, g, b } => parts.push(format!("48;2;{r};{g};{b}")),
    }
}

/// Push an underline color into the style parts.
fn push_ul_color(parts: &mut Vec<String>, color: Color) {
    match color {
        Color::NoColor => {}
        Color::Basic(n) => parts.push(format!("58;5;{n}")),
        Color::Indexed(n) => parts.push(format!("58;5;{n}")),
        Color::Rgb { r, g, b } => parts.push(format!("58;2;{r};{g};{b}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_to_profile(input: &str, profile: Profile) -> String {
        let mut buf = Vec::new();
        let mut w = ProfileWriter::new(&mut buf, profile);
        w.write_all(input.as_bytes()).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn test_empty() {
        assert_eq!(write_to_profile("", Profile::TrueColor), "");
        assert_eq!(write_to_profile("", Profile::Ansi256), "");
        assert_eq!(write_to_profile("", Profile::Ansi), "");
        assert_eq!(write_to_profile("", Profile::Ascii), "");
        assert_eq!(write_to_profile("", Profile::NoTty), "");
    }

    #[test]
    fn test_no_styles() {
        let input = "hello world";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(write_to_profile(input, Profile::Ansi256), input);
        assert_eq!(write_to_profile(input, Profile::Ansi), input);
        assert_eq!(write_to_profile(input, Profile::Ascii), input);
        assert_eq!(write_to_profile(input, Profile::NoTty), input);
    }

    #[test]
    fn test_simple_style_attributes() {
        let input = "hello \x1b[1mworld\x1b[m";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "hello \x1b[1mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "hello \x1b[1mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "hello \x1b[1mworld\x1b[m"
        );
    }

    #[test]
    fn test_simple_ansi_color_fg() {
        let input = "hello \x1b[31mworld\x1b[m";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "hello \x1b[31mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "hello \x1b[31mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "hello \x1b[mworld\x1b[m"
        );
    }

    #[test]
    fn test_default_fg_after_ansi_color() {
        let input = "\x1b[31mhello \x1b[39mworld\x1b[m";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "\x1b[31mhello \x1b[39mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "\x1b[31mhello \x1b[39mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "\x1b[mhello \x1b[mworld\x1b[m"
        );
    }

    #[test]
    fn test_ansi_color_fg_and_bg() {
        let input = "\x1b[31;42mhello world\x1b[m";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "\x1b[31;42mhello world\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "\x1b[31;42mhello world\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "\x1b[mhello world\x1b[m"
        );
    }

    #[test]
    fn test_bright_ansi_fg_and_bg() {
        let input = "\x1b[91;102mhello world\x1b[m";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "\x1b[91;102mhello world\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "\x1b[91;102mhello world\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "\x1b[mhello world\x1b[m"
        );
    }

    #[test]
    fn test_256_color_fg() {
        let input = "hello \x1b[38;5;196mworld\x1b[m";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "hello \x1b[38;5;196mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "hello \x1b[91mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "hello \x1b[mworld\x1b[m"
        );
    }

    #[test]
    fn test_256_color_bg() {
        let input = "\x1b[48;5;196mhello world\x1b[m";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "\x1b[48;5;196mhello world\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "\x1b[101mhello world\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "\x1b[mhello world\x1b[m"
        );
    }

    #[test]
    fn test_truecolor_fg() {
        // #ff8537 -> 256: 209, 16: 9 (bright red)
        let input = "hello \x1b[38;2;255;133;55mworld\x1b[m";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "hello \x1b[38;5;209mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "hello \x1b[91mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "hello \x1b[mworld\x1b[m"
        );
    }

    #[test]
    fn test_itu_truecolor_bg() {
        // Colon-separated: 38:2::255:133:55
        let input = "hello \x1b[38:2::255:133:55mworld\x1b[m";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "hello \x1b[38;5;209mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "hello \x1b[91mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "hello \x1b[mworld\x1b[m"
        );
    }

    #[test]
    fn test_itu_256_color_bg() {
        let input = "hello \x1b[48:5:196mworld\x1b[m";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "hello \x1b[48;5;196mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "hello \x1b[101mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "hello \x1b[mworld\x1b[m"
        );
    }

    #[test]
    fn test_missing_param() {
        let input = "\x1b[31mhello \x1b[;1mworld";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "\x1b[31mhello \x1b[;1mworld"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "\x1b[31mhello \x1b[;1mworld"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "\x1b[mhello \x1b[;1mworld"
        );
    }

    #[test]
    fn test_color_with_other_attributes() {
        let input = "\x1b[1;38;5;204mhello \x1b[38;5;204mworld\x1b[m";
        assert_eq!(write_to_profile(input, Profile::TrueColor), input);
        assert_eq!(
            write_to_profile(input, Profile::Ansi256),
            "\x1b[1;38;5;204mhello \x1b[38;5;204mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ansi),
            "\x1b[1;91mhello \x1b[91mworld\x1b[m"
        );
        assert_eq!(
            write_to_profile(input, Profile::Ascii),
            "\x1b[1mhello \x1b[mworld\x1b[m"
        );
    }

    #[test]
    fn test_notty_strips_all() {
        let input = "hello \x1b[31mworld\x1b[m";
        assert_eq!(write_to_profile(input, Profile::NoTty), "hello world");
    }

    #[test]
    fn test_notty_strips_complex() {
        let input = "\x1b[1;38;5;204mhello \x1b[38;5;204mworld\x1b[m";
        assert_eq!(write_to_profile(input, Profile::NoTty), "hello world");
    }
}
