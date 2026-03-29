use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rouge_harmonica::{Spring, fps};
use rouge_runtime::{Cmd, Msg};
use rouge_style::Color;

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

        let full_style = rouge_style::Style::new().foreground(self.full_color);
        let empty_style = rouge_style::Style::new().foreground(self.empty_color);

        let full_str: String = std::iter::repeat_n(self.full_char, filled).collect();
        let empty_str: String = std::iter::repeat_n(self.empty_char, empty).collect();

        let bar = format!(
            "{}{}",
            full_style.render(&[&full_str]),
            empty_style.render(&[&empty_str])
        );

        if self.show_percentage {
            format!("{} {:>3.0}%", bar, pct * 100.0)
        } else {
            bar
        }
    }

    fn frame_cmd(&self) -> Cmd {
        let dur = Duration::from_millis(1000 / ANIM_FPS as u64);
        let id = self.id;
        let tag = self.tag;
        rouge_runtime::tick(dur, move |_| Msg::custom(ProgressFrameMsg { id, tag }))
    }
}
