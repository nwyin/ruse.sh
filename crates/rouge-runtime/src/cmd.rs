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
