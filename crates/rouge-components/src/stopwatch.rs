use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rouge_runtime::{Cmd, Msg};

static STOPWATCH_ID: AtomicUsize = AtomicUsize::new(0);

fn next_id() -> usize {
    STOPWATCH_ID.fetch_add(1, Ordering::Relaxed)
}

/// Sent on each tick interval while the stopwatch is running.
pub struct StopwatchTickMsg {
    pub id: usize,
    tag: usize,
}

pub struct Stopwatch {
    elapsed: Duration,
    interval: Duration,
    id: usize,
    tag: usize,
    running: bool,
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::new()
    }
}

impl Stopwatch {
    pub fn new() -> Self {
        Self {
            elapsed: Duration::ZERO,
            interval: Duration::from_secs(1),
            id: next_id(),
            tag: 0,
            running: false,
        }
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn start(&mut self) -> Cmd {
        self.running = true;
        self.tag += 1;
        self.tick_cmd()
    }

    pub fn stop(&mut self) -> Cmd {
        self.running = false;
        self.tag += 1;
        None
    }

    pub fn toggle(&mut self) -> Cmd {
        if self.running {
            self.stop()
        } else {
            self.start()
        }
    }

    pub fn reset(&mut self) -> Cmd {
        self.elapsed = Duration::ZERO;
        self.tag += 1;
        if self.running {
            self.tick_cmd()
        } else {
            None
        }
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        if let Some(tick) = msg.downcast_ref::<StopwatchTickMsg>() {
            if tick.id != self.id || tick.tag != self.tag {
                return None;
            }
            if !self.running {
                return None;
            }
            self.elapsed += self.interval;
            return self.tick_cmd();
        }
        None
    }

    pub fn view(&self) -> String {
        let secs = self.elapsed.as_secs();
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        let s = secs % 60;
        if hours > 0 {
            format!("{hours:02}:{mins:02}:{s:02}")
        } else {
            format!("{mins:02}:{s:02}")
        }
    }

    fn tick_cmd(&self) -> Cmd {
        let dur = self.interval;
        let id = self.id;
        let tag = self.tag;
        rouge_runtime::tick(dur, move |_| Msg::custom(StopwatchTickMsg { id, tag }))
    }
}
