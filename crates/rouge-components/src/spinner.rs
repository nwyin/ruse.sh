use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rouge_runtime::{Cmd, Msg};
use rouge_style::Style;

static SPINNER_ID: AtomicUsize = AtomicUsize::new(0);

fn next_id() -> usize {
    SPINNER_ID.fetch_add(1, Ordering::Relaxed)
}

/// Defines the frames and speed of a spinner animation.
pub struct SpinnerFrames {
    pub frames: Vec<&'static str>,
    pub fps: u64, // milliseconds per frame
}

pub fn line_spinner() -> SpinnerFrames {
    SpinnerFrames {
        frames: vec!["|", "/", "-", "\\"],
        fps: 100,
    }
}

pub fn dot_spinner() -> SpinnerFrames {
    SpinnerFrames {
        frames: vec!["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"],
        fps: 100,
    }
}

pub fn mini_dot_spinner() -> SpinnerFrames {
    SpinnerFrames {
        frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
        fps: 100,
    }
}

pub fn pulse_spinner() -> SpinnerFrames {
    SpinnerFrames {
        frames: vec!["░", "▒", "▓", "█", "▓", "▒", "░"],
        fps: 100,
    }
}

pub fn globe_spinner() -> SpinnerFrames {
    SpinnerFrames {
        frames: vec!["🌍", "🌎", "🌏"],
        fps: 200,
    }
}

pub fn moon_spinner() -> SpinnerFrames {
    SpinnerFrames {
        frames: vec!["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘"],
        fps: 150,
    }
}

pub fn ellipsis_spinner() -> SpinnerFrames {
    SpinnerFrames {
        frames: vec![".", "..", "..."],
        fps: 300,
    }
}

struct TickMsg {
    id: usize,
    tag: usize,
}

/// A spinner component that animates through frames.
pub struct Spinner {
    frames: SpinnerFrames,
    frame: usize,
    style: Style,
    id: usize,
    tag: usize,
}

impl Spinner {
    pub fn new(frames: SpinnerFrames) -> Self {
        Self {
            frames,
            frame: 0,
            style: Style::new(),
            id: next_id(),
            tag: 0,
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Returns the initial tick command to start the spinner animation.
    pub fn init(&self) -> Cmd {
        let dur = Duration::from_millis(self.frames.fps);
        let id = self.id;
        let tag = self.tag;
        rouge_runtime::tick(dur, move |_| Msg::custom(TickMsg { id, tag }))
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        if let Some(tick) = msg.downcast_ref::<TickMsg>() {
            if tick.id != self.id || tick.tag != self.tag {
                return None;
            }
            self.frame = (self.frame + 1) % self.frames.frames.len();
            return self.init();
        }
        None
    }

    pub fn view(&self) -> String {
        let frame_str = self.frames.frames[self.frame % self.frames.frames.len()];
        self.style.render(&[frame_str])
    }
}
