use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use ruse_runtime::{Cmd, Msg};

static TIMER_ID: AtomicUsize = AtomicUsize::new(0);

fn next_id() -> usize {
    TIMER_ID.fetch_add(1, Ordering::Relaxed)
}

/// Sent on each tick interval while the timer is running.
pub struct TimerTickMsg {
    pub id: usize,
    tag: usize,
}

/// Sent when the timer reaches its timeout.
pub struct TimerTimeoutMsg {
    pub id: usize,
}

/// Sent when the timer starts or stops.
pub struct TimerStartStopMsg {
    pub id: usize,
    pub running: bool,
}

pub struct Timer {
    pub timeout: Duration,
    pub interval: Duration,
    id: usize,
    tag: usize,
    running: bool,
    elapsed: Duration,
}

impl Timer {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            interval: Duration::from_secs(1),
            id: next_id(),
            tag: 0,
            running: false,
            elapsed: Duration::ZERO,
        }
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub fn running(&self) -> bool {
        self.running
    }

    pub fn timed_out(&self) -> bool {
        self.elapsed >= self.timeout
    }

    pub fn start(&mut self) -> Cmd {
        self.running = true;
        self.tag += 1;
        let cmds = vec![self.tick_cmd(), self.start_stop_cmd(true)];
        ruse_runtime::batch(cmds)
    }

    pub fn stop(&mut self) -> Cmd {
        self.running = false;
        self.tag += 1;
        self.start_stop_cmd(false)
    }

    pub fn toggle(&mut self) -> Cmd {
        if self.running {
            self.stop()
        } else {
            self.start()
        }
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        if let Some(tick) = msg.downcast_ref::<TimerTickMsg>() {
            if tick.id != self.id || tick.tag != self.tag {
                return None;
            }
            if !self.running {
                return None;
            }
            self.elapsed += self.interval;
            if self.elapsed >= self.timeout {
                self.running = false;
                return self.timeout_cmd();
            }
            return self.tick_cmd();
        }
        None
    }

    pub fn view(&self) -> String {
        let remaining = self.timeout.saturating_sub(self.elapsed);
        let secs = remaining.as_secs();
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
        ruse_runtime::tick(dur, move |_| Msg::custom(TimerTickMsg { id, tag }))
    }

    fn timeout_cmd(&self) -> Cmd {
        let id = self.id;
        ruse_runtime::cmd(move || Msg::custom(TimerTimeoutMsg { id }))
    }

    fn start_stop_cmd(&self, running: bool) -> Cmd {
        let id = self.id;
        ruse_runtime::cmd(move || Msg::custom(TimerStartStopMsg { id, running }))
    }
}
