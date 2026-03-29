use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rouge_runtime::{Cmd, Msg};
use rouge_style::Style;

static CURSOR_ID: AtomicUsize = AtomicUsize::new(0);

fn next_id() -> usize {
    CURSOR_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Copy, PartialEq)]
pub enum CursorMode {
    Blink,
    Static,
    Hidden,
}

struct BlinkMsg {
    id: usize,
    tag: usize,
}

const DEFAULT_BLINK_SPEED: u64 = 530;

pub struct Cursor {
    style: Style,
    text_style: Style,
    blink_speed_ms: u64,
    blinked: bool,
    char: String,
    id: usize,
    focus: bool,
    mode: CursorMode,
    tag: usize,
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            style: Style::new().reverse(true),
            text_style: Style::new(),
            blink_speed_ms: DEFAULT_BLINK_SPEED,
            blinked: false,
            char: " ".to_string(),
            id: next_id(),
            focus: false,
            mode: CursorMode::Blink,
            tag: 0,
        }
    }

    pub fn with_style(mut self, s: Style) -> Self {
        self.style = s;
        self
    }

    pub fn set_char(&mut self, ch: &str) {
        self.char = if ch.is_empty() {
            " ".to_string()
        } else {
            ch.to_string()
        };
    }

    pub fn set_mode(&mut self, mode: CursorMode) -> Cmd {
        self.mode = mode;
        if mode == CursorMode::Blink && self.focus {
            self.blinked = false;
            return self.blink_cmd();
        }
        None
    }

    pub fn focus(&mut self) -> Cmd {
        self.focus = true;
        self.blinked = false;
        self.tag += 1;
        if self.mode == CursorMode::Blink {
            return self.blink_cmd();
        }
        None
    }

    pub fn blur(&mut self) {
        self.focus = false;
        self.blinked = false;
    }

    pub fn focused(&self) -> bool {
        self.focus
    }

    pub fn update(&mut self, msg: &Msg) -> Cmd {
        if let Some(blink_msg) = msg.downcast_ref::<BlinkMsg>() {
            if blink_msg.id != self.id || blink_msg.tag != self.tag {
                return None;
            }
            if self.mode != CursorMode::Blink || !self.focus {
                return None;
            }
            self.blinked = !self.blinked;
            return self.blink_cmd();
        }
        None
    }

    pub fn view(&self) -> String {
        if !self.focus || self.mode == CursorMode::Hidden {
            return self.text_style.render(&[&self.char]);
        }
        if self.mode == CursorMode::Blink && self.blinked {
            return self.text_style.render(&[&self.char]);
        }
        self.style.render(&[&self.char])
    }

    fn blink_cmd(&self) -> Cmd {
        let dur = Duration::from_millis(self.blink_speed_ms);
        let id = self.id;
        let tag = self.tag;
        rouge_runtime::tick(dur, move |_| Msg::custom(BlinkMsg { id, tag }))
    }
}
