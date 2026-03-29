use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use ruse_harmonica::{Spring, fps};
use ruse_runtime::{Cmd, Msg};
use ruse_style::Color;

static PROGRESS_ID: AtomicUsize = AtomicUsize::new(0);

fn next_id() -> usize {
    PROGRESS_ID.fetch_add(1, Ordering::Relaxed)
}

struct ProgressFrameMsg {
    id: usize,
    tag: usize,
}

const ANIM_FPS: u32 = 60;
const DEFAULT_WIDTH: usize = 40;

pub struct Progress {
    pub width: usize,
    pub full_char: char,
    pub empty_char: char,
    pub full_color: Color,
    pub empty_color: Color,
    pub show_percentage: bool,
    percent: f64,
    target_percent: f64,
    velocity: f64,
    spring: Spring,
    id: usize,
    tag: usize,
    gradient_colors: Vec<Color>,
    scale_gradient: bool,
}

impl Default for Progress {
    fn default() -> Self {
        Self::new()
    }
}

impl Progress {
    pub fn new() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            full_char: '█',
            empty_char: '░',
            full_color: Color::Rgb { r: 100, g: 200, b: 100 },
            empty_color: Color::Rgb { r: 80, g: 80, b: 80 },
            show_percentage: true,
            percent: 0.0,
            target_percent: 0.0,
            velocity: 0.0,
            spring: Spring::new(fps(ANIM_FPS), 6.0, 1.0),
            id: next_id(),
            tag: 0,
            gradient_colors: Vec::new(),
            scale_gradient: false,
        }
    }

    pub fn with_width(mut self, w: usize) -> Self {
        self.width = w;
        self
    }

    pub fn with_colors(mut self, full: Color, empty: Color) -> Self {
        self.full_color = full;
        self.empty_color = empty;
        self
    }

    /// Set a gradient of colors for the filled portion of the progress bar.
    /// Colors are spread evenly across the filled width. If `scale_gradient`
    /// is true, the gradient always covers the full bar width; otherwise it
    /// covers only the filled portion.
    pub fn with_gradient(mut self, colors: Vec<Color>) -> Self {
        self.gradient_colors = colors;
        self
    }

    /// When true the gradient stretches over the full bar width and is
    /// revealed progressively.  When false the gradient is rescaled to
    /// cover only the filled portion.
    pub fn with_scale_gradient(mut self, v: bool) -> Self {
        self.scale_gradient = v;
        self
    }

    /// Set the target percentage (0.0..1.0) and start the animation.
    pub fn set_percent(&mut self, p: f64) -> Cmd {
        self.target_percent = p.clamp(0.0, 1.0);
        self.tag += 1;
        self.frame_cmd()
    }

    pub fn percent(&self) -> f64 {
        self.percent
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        if let Some(frame) = msg.downcast_ref::<ProgressFrameMsg>() {
            if frame.id != self.id || frame.tag != self.tag {
                return None;
            }
            let (new_pos, new_vel) = self.spring.update(self.percent, self.velocity, self.target_percent);
            self.percent = new_pos;
            self.velocity = new_vel;

            // Stop animating when close enough
            if (self.percent - self.target_percent).abs() < 0.001 && self.velocity.abs() < 0.001 {
                self.percent = self.target_percent;
                self.velocity = 0.0;
                return None;
            }
            return self.frame_cmd();
        }
        None
    }

    pub fn view(&self) -> String {
        let pct = self.percent.clamp(0.0, 1.0);
        let filled = (pct * self.width as f64).round() as usize;
        let empty = self.width.saturating_sub(filled);

        let empty_style = ruse_style::Style::new().foreground(self.empty_color);
        let empty_str: String = std::iter::repeat_n(self.empty_char, empty).collect();

        let bar = if !self.gradient_colors.is_empty() && filled > 0 {
            // Render each filled cell with a gradient color
            let mut full_part = String::new();
            for i in 0..filled {
                let color = self.gradient_color_at(i, filled);
                let cell_style = ruse_style::Style::new().foreground(color);
                full_part.push_str(&cell_style.render(&[&self.full_char.to_string()]));
            }
            format!("{}{}", full_part, empty_style.render(&[&empty_str]))
        } else {
            let full_style = ruse_style::Style::new().foreground(self.full_color);
            let full_str: String = std::iter::repeat_n(self.full_char, filled).collect();
            format!(
                "{}{}",
                full_style.render(&[&full_str]),
                empty_style.render(&[&empty_str])
            )
        };

        if self.show_percentage {
            format!("{} {:>3.0}%", bar, pct * 100.0)
        } else {
            bar
        }
    }

    /// Compute the gradient color for position `i` in the filled region of
    /// size `filled`.
    fn gradient_color_at(&self, i: usize, filled: usize) -> Color {
        let colors = &self.gradient_colors;
        if colors.is_empty() {
            return self.full_color;
        }
        if colors.len() == 1 {
            return colors[0];
        }

        // Determine the reference width for interpolation
        let ref_width = if self.scale_gradient { self.width } else { filled };
        if ref_width <= 1 {
            return colors[0];
        }

        let t = i as f64 / (ref_width - 1) as f64;
        let segment_count = colors.len() - 1;
        let raw_idx = t * segment_count as f64;
        let seg = (raw_idx as usize).min(segment_count - 1);
        let seg_t = raw_idx - seg as f64;

        lerp_color(colors[seg], colors[seg + 1], seg_t)
    }

    fn frame_cmd(&self) -> Cmd {
        let dur = Duration::from_millis(1000 / ANIM_FPS as u64);
        let id = self.id;
        let tag = self.tag;
        ruse_runtime::tick(dur, move |_| Msg::custom(ProgressFrameMsg { id, tag }))
    }
}

/// Linearly interpolate between two colors. Falls back to `a` if either
/// color is not RGB.
fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    match (a, b) {
        (Color::Rgb { r: r1, g: g1, b: b1 }, Color::Rgb { r: r2, g: g2, b: b2 }) => {
            let t = t.clamp(0.0, 1.0);
            Color::Rgb {
                r: (r1 as f64 + (r2 as f64 - r1 as f64) * t).round() as u8,
                g: (g1 as f64 + (g2 as f64 - g1 as f64) * t).round() as u8,
                b: (b1 as f64 + (b2 as f64 - b1 as f64) * t).round() as u8,
            }
        }
        _ => a,
    }
}
