use std::future::Future;
use std::pin::Pin;

use crate::msg::Msg;

/// A command is an optional action that produces a message.
/// `None` means "no command" (do nothing).
pub type Cmd = Option<CmdInner>;

/// The inner command type: either a synchronous closure or an async future.
pub enum CmdInner {
    Sync(Box<dyn FnOnce() -> Msg + Send + 'static>),
    Async(Pin<Box<dyn Future<Output = Msg> + Send + 'static>>),
}

/// Create a synchronous command from a closure.
pub fn cmd<F: FnOnce() -> Msg + Send + 'static>(f: F) -> Cmd {
    Some(CmdInner::Sync(Box::new(f)))
}

/// Create an asynchronous command from a future.
pub fn cmd_async<F: Future<Output = Msg> + Send + 'static>(fut: F) -> Cmd {
    Some(CmdInner::Async(Box::pin(fut)))
}

/// Run multiple commands concurrently (no ordering guarantees).
pub fn batch(cmds: Vec<Cmd>) -> Cmd {
    let valid: Vec<CmdInner> = cmds.into_iter().flatten().collect();
    match valid.len() {
        0 => None,
        1 => Some(valid.into_iter().next().unwrap()),
        _ => Some(CmdInner::Sync(Box::new(|| {
            Msg::Batch(valid.into_iter().map(Some).collect())
        }))),
    }
}

/// Run multiple commands sequentially (one at a time, in order).
pub fn sequence(cmds: Vec<Cmd>) -> Cmd {
    let valid: Vec<CmdInner> = cmds.into_iter().flatten().collect();
    match valid.len() {
        0 => None,
        1 => Some(valid.into_iter().next().unwrap()),
        _ => Some(CmdInner::Sync(Box::new(|| {
            Msg::Sequence(valid.into_iter().map(Some).collect())
        }))),
    }
}

/// Command that causes the program to quit.
pub fn quit() -> Cmd {
    cmd(|| Msg::Quit)
}

/// Command that sleeps for the given duration, then calls `f` with the current instant.
pub fn tick(
    duration: std::time::Duration,
    f: impl FnOnce(std::time::Instant) -> Msg + Send + 'static,
) -> Cmd {
    cmd_async(async move {
        tokio::time::sleep(duration).await;
        f(std::time::Instant::now())
    })
}

/// Command that sends a `PrintLine` message with the given string.
pub fn println(s: impl Into<String>) -> Cmd {
    let s = s.into();
    cmd(move || Msg::PrintLine(s))
}

/// Command that sends a `ClearScreen` message.
pub fn clear_screen() -> Cmd {
    cmd(|| Msg::ClearScreen)
}

/// Command that ticks in sync with the system clock.
/// E.g., `every(Duration::from_secs(1), |t| Msg::Tick(t))` ticks at :00, :01, :02.
pub fn every(
    duration: std::time::Duration,
    f: impl FnOnce(std::time::SystemTime) -> Msg + Send + 'static,
) -> Cmd {
    cmd_async(async move {
        let now = std::time::SystemTime::now();
        let epoch_ms = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let dur_ms = duration.as_millis().max(1);
        let next_boundary_ms = ((epoch_ms / dur_ms) + 1) * dur_ms;
        let wait = std::time::Duration::from_millis((next_boundary_ms - epoch_ms) as u64);
        tokio::time::sleep(wait).await;
        f(std::time::SystemTime::now())
    })
}

/// Command to execute an external process (e.g., vim, shell).
/// The terminal is released before exec and restored after.
/// The callback receives the exit status.
pub fn exec<F>(
    program: impl Into<String> + Send + 'static,
    args: Vec<String>,
    on_finish: F,
) -> Cmd
where
    F: FnOnce(std::io::Result<std::process::ExitStatus>) -> Msg + Send + 'static,
{
    let program = program.into();
    Some(CmdInner::Sync(Box::new(move || {
        Msg::custom(ExecRequest {
            program,
            args,
            callback: Box::new(on_finish),
        })
    })))
}

/// Internal exec request message.
pub(crate) struct ExecRequest {
    pub program: String,
    pub args: Vec<String>,
    pub callback: Box<dyn FnOnce(std::io::Result<std::process::ExitStatus>) -> Msg + Send>,
}

/// Command to inject a raw escape sequence into the terminal output.
pub fn raw(sequence: impl Into<String> + Send + 'static) -> Cmd {
    let seq = sequence.into();
    cmd(move || Msg::custom(RawSequence(seq)))
}

/// Internal raw sequence message.
pub(crate) struct RawSequence(pub String);

/// Command to set the system clipboard via OSC 52.
pub fn set_clipboard(content: impl Into<String> + Send + 'static) -> Cmd {
    let content = content.into();
    let encoded = base64_encode(content.as_bytes());
    raw(format!("\x1b]52;c;{}\x07", encoded))
}

/// Command to request the system clipboard via OSC 52.
pub fn read_clipboard() -> Cmd {
    raw("\x1b]52;c;?\x07")
}

/// Simple base64 encoding (no external dependency needed).
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}
