use std::collections::hash_map::DefaultHasher;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::Arc;

use crossterm::{cursor, event, event::EventStream, execute, queue, terminal};
use futures::StreamExt;
use ruse_ansi::cellbuf;
use tokio::sync::{Notify, mpsc};
use tokio_util::sync::CancellationToken;

use crate::cmd::{CmdInner, ExecRequest, RawSequence};
use crate::error::ProgramError;
use crate::input::translate_event;
use crate::model::Model;
use crate::msg::Msg;
use crate::view::{CursorShape, MouseMode, View};

/// Drop guard that restores terminal state on exit (including panics).
struct TerminalGuard {
    alt_screen: bool,
    mouse_mode: MouseMode,
    report_focus: bool,
    bracketed_paste: bool,
    kitty_keyboard: bool,
    raw_mode: bool,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();

        if self.kitty_keyboard {
            let _ = execute!(stdout, event::PopKeyboardEnhancementFlags);
        }

        if self.bracketed_paste {
            let _ = execute!(stdout, event::DisableBracketedPaste);
        }

        if self.report_focus {
            let _ = execute!(stdout, event::DisableFocusChange);
        }

        match self.mouse_mode {
            MouseMode::CellMotion | MouseMode::AllMotion => {
                let _ = execute!(stdout, event::DisableMouseCapture);
            }
            MouseMode::None => {}
        }

        let _ = execute!(stdout, cursor::Show);

        if self.alt_screen {
            let _ = execute!(stdout, terminal::LeaveAlternateScreen);
        }

        if self.raw_mode {
            let _ = terminal::disable_raw_mode();
        }
    }
}

/// Handle for controlling a running Program from outside.
#[derive(Clone)]
pub struct ProgramHandle {
    msg_tx: mpsc::UnboundedSender<Msg>,
    cancel: CancellationToken,
    finished: Arc<Notify>,
}

impl ProgramHandle {
    /// Send a message to the running program.
    pub fn send(&self, msg: Msg) -> Result<(), &'static str> {
        if self.cancel.is_cancelled() {
            return Err("program already stopped");
        }
        self.msg_tx.send(msg).map_err(|_| "channel closed")
    }

    /// Request graceful shutdown (final render happens).
    pub fn quit(&self) {
        let _ = self.send(Msg::Quit);
    }

    /// Force immediate shutdown (skip final render).
    pub fn kill(&self) {
        self.cancel.cancel();
    }

    /// Wait for the program to finish.
    pub async fn wait(&self) {
        self.finished.notified().await;
    }
}

/// Message filter function type.
pub type MessageFilter<M> = Box<dyn Fn(&M, Msg) -> Option<Msg> + Send>;

/// The main runtime for an Elm-architecture TUI program.
pub struct Program<M: Model> {
    model: M,
    fps: u32,
    alt_screen: bool,
    mouse_mode: MouseMode,
    report_focus: bool,
    bracketed_paste: bool,
    kitty_keyboard: bool,
    disable_renderer: bool,
    #[allow(dead_code)]
    disable_signals: bool,
    filter: Option<MessageFilter<M>>,
    external_cancel: Option<CancellationToken>,
}

impl<M: Model> Program<M> {
    /// Create a new program with the given model.
    pub fn new(model: M) -> Self {
        Self {
            model,
            fps: 60,
            alt_screen: false,
            mouse_mode: MouseMode::None,
            report_focus: false,
            bracketed_paste: false,
            kitty_keyboard: false,
            disable_renderer: false,
            disable_signals: false,
            filter: None,
            external_cancel: None,
        }
    }

    /// Set the maximum frames per second for rendering.
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps.clamp(1, 120);
        self
    }

    /// Start in alternate screen mode.
    pub fn with_alt_screen(mut self) -> Self {
        self.alt_screen = true;
        self
    }

    /// Set the mouse mode.
    pub fn with_mouse(mut self, mode: MouseMode) -> Self {
        self.mouse_mode = mode;
        self
    }

    /// Enable focus reporting.
    pub fn with_focus_report(mut self) -> Self {
        self.report_focus = true;
        self
    }

    /// Enable bracketed paste mode.
    ///
    /// When enabled, pasted text arrives as a single `Msg::Paste(String)`
    /// instead of individual key events. This also suppresses macOS paste
    /// confirmation dialogs for multi-line content.
    pub fn with_bracketed_paste(mut self) -> Self {
        self.bracketed_paste = true;
        self
    }

    /// Enable Kitty keyboard protocol for enhanced key reporting.
    ///
    /// This allows distinguishing modifier combinations that standard terminal
    /// input cannot differentiate, such as Shift+Enter vs Enter.
    pub fn with_kitty_keyboard(mut self) -> Self {
        self.kitty_keyboard = true;
        self
    }

    /// Disable the renderer (plain output mode).
    pub fn without_renderer(mut self) -> Self {
        self.disable_renderer = true;
        self
    }

    /// Disable the signal handler.
    pub fn without_signal_handler(mut self) -> Self {
        self.disable_signals = true;
        self
    }

    /// Set a message filter that intercepts messages before model.update().
    /// Return `Some(msg)` to pass through, `None` to drop the message.
    pub fn with_filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&M, Msg) -> Option<Msg> + Send + 'static,
    {
        self.filter = Some(Box::new(filter));
        self
    }

    /// Set an external cancellation token for program control.
    pub fn with_context(mut self, token: CancellationToken) -> Self {
        self.external_cancel = Some(token);
        self
    }

    /// Run the program, blocking until it exits.
    /// Returns the final model state on success.
    pub async fn run(self) -> Result<M, ProgramError> {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<Msg>();
        let cancel = CancellationToken::new();
        self.run_with_panic_recovery(msg_tx, msg_rx, cancel, None)
            .await
    }

    /// Run the program, returning a handle for external message injection.
    /// The handle can send messages into the program's event loop from any thread.
    /// The returned future must be awaited to actually run the program.
    #[allow(clippy::type_complexity)]
    pub fn run_with_handle(
        self,
    ) -> (
        ProgramHandle,
        Pin<Box<dyn Future<Output = Result<M, ProgramError>> + Send>>,
    ) {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel::<Msg>();
        let cancel = CancellationToken::new();
        let finished = Arc::new(Notify::new());

        let handle = ProgramHandle {
            msg_tx: msg_tx.clone(),
            cancel: cancel.clone(),
            finished: finished.clone(),
        };

        let fut = Box::pin(async move {
            let result = self
                .run_with_panic_recovery(msg_tx, msg_rx, cancel, Some(finished.clone()))
                .await;
            finished.notify_waiters();
            result
        });

        (handle, fut)
    }

    /// Shared panic-recovery wrapper.
    async fn run_with_panic_recovery(
        self,
        msg_tx: mpsc::UnboundedSender<Msg>,
        msg_rx: mpsc::UnboundedReceiver<Msg>,
        cancel: CancellationToken,
        _finished: Option<Arc<Notify>>,
    ) -> Result<M, ProgramError> {
        match tokio::task::spawn(AssertUnwindSafe(self.run_inner(msg_tx, msg_rx, cancel))).await {
            Ok(result) => result,
            Err(join_err) => {
                restore_terminal_emergency();
                if join_err.is_panic() {
                    let panic_msg = if let Ok(s) = join_err.try_into_panic() {
                        if let Some(s) = s.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = s.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "unknown panic".to_string()
                        }
                    } else {
                        "unknown panic".to_string()
                    };

                    if std::env::var("TEA_DEBUG")
                        .ok()
                        .and_then(|v| v.parse::<bool>().ok())
                        == Some(true)
                    {
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        let path = format!("ruse-panic-{}.log", ts);
                        let _ = std::fs::write(&path, &panic_msg);
                    }

                    Err(ProgramError::Panic(panic_msg))
                } else {
                    Err(ProgramError::Killed)
                }
            }
        }
    }

    /// Inner run implementation (may panic, caught by outer wrapper).
    async fn run_inner(
        mut self,
        msg_tx: mpsc::UnboundedSender<Msg>,
        mut msg_rx: mpsc::UnboundedReceiver<Msg>,
        cancel: CancellationToken,
    ) -> Result<M, ProgramError> {
        // Enable raw mode
        terminal::enable_raw_mode()?;

        let mut stdout = io::stdout();
        let mut current_alt_screen = false;
        let mut current_mouse_mode = MouseMode::None;
        let mut current_report_focus = false;

        // Set up initial terminal state
        if self.alt_screen {
            execute!(stdout, terminal::EnterAlternateScreen)?;
            current_alt_screen = true;
        }

        match self.mouse_mode {
            MouseMode::CellMotion | MouseMode::AllMotion => {
                execute!(stdout, event::EnableMouseCapture)?;
                current_mouse_mode = self.mouse_mode;
            }
            MouseMode::None => {}
        }

        if self.report_focus {
            execute!(stdout, event::EnableFocusChange)?;
            current_report_focus = true;
        }

        let mut current_bracketed_paste = false;
        if self.bracketed_paste {
            execute!(stdout, event::EnableBracketedPaste)?;
            current_bracketed_paste = true;
        }

        let mut current_kitty_keyboard = false;
        if self.kitty_keyboard {
            let _ = execute!(
                stdout,
                event::PushKeyboardEnhancementFlags(
                    event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                )
            );
            current_kitty_keyboard = true;
        }

        // Install the drop guard for cleanup
        let guard = TerminalGuard {
            alt_screen: current_alt_screen,
            mouse_mode: current_mouse_mode,
            report_focus: current_report_focus,
            bracketed_paste: current_bracketed_paste,
            kitty_keyboard: current_kitty_keyboard,
            raw_mode: true,
        };

        // Set up the cell-buffer based screen renderer
        let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
        let mut screen = cellbuf::Screen::new(term_w, term_h);
        let mut last_view_hash: Option<u64> = None;
        let mut render_dirty = false;

        // Combine external cancel token if provided
        if let Some(ext) = &self.external_cancel {
            let internal = cancel.clone();
            let ext = ext.clone();
            tokio::spawn(async move {
                ext.cancelled().await;
                internal.cancel();
            });
        }

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<CmdInner>();

        // Call model.init() and submit any returned command
        if let Some(cmd_inner) = self.model.init() {
            let _ = cmd_tx.send(cmd_inner);
        }

        // Send initial WindowSize so the model can size itself on first frame
        if let Some(cmd_inner) = self.model.update(Msg::WindowSize {
            width: term_w,
            height: term_h,
        }) {
            let _ = cmd_tx.send(cmd_inner);
        }

        // Render initial view
        if !self.disable_renderer {
            let view = self.model.view();
            apply_view_state(
                &mut stdout,
                &view,
                &mut current_alt_screen,
                &mut current_mouse_mode,
                &mut current_report_focus,
            )?;
            render_view(&mut screen, &view);
            screen.render(&mut stdout)?;
            render_cursor(&mut stdout, &view)?;
            last_view_hash = Some(hash_string(&view.content));
        }

        // Spawn input reader task
        let input_cancel = cancel.clone();
        let input_msg_tx = msg_tx.clone();
        let input_handle = tokio::spawn(async move {
            let mut reader = EventStream::new();
            loop {
                tokio::select! {
                    _ = input_cancel.cancelled() => break,
                    maybe_event = reader.next() => {
                        match maybe_event {
                            Some(Ok(event)) => {
                                if let Some(msg) = translate_event(event)
                                    && input_msg_tx.send(msg).is_err() {
                                        break;
                                    }
                            }
                            Some(Err(_)) => break,
                            None => break,
                        }
                    }
                }
            }
        });

        // Spawn command handler task
        let cmd_cancel = cancel.clone();
        let cmd_msg_tx = msg_tx.clone();
        let cmd_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cmd_cancel.cancelled() => break,
                    maybe_cmd = cmd_rx.recv() => {
                        match maybe_cmd {
                            Some(cmd_inner) => {
                                let tx = cmd_msg_tx.clone();
                                tokio::spawn(async move {
                                    let msg = execute_cmd(cmd_inner).await;
                                    let _ = tx.send(msg);
                                });
                            }
                            None => break,
                        }
                    }
                }
            }
        });

        // Spawn render ticker task
        let render_cancel = cancel.clone();
        let render_msg_tx = msg_tx.clone();
        let fps = self.fps;
        let render_handle = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_micros(1_000_000 / fps as u64));
            loop {
                tokio::select! {
                    _ = render_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        let _ = &render_msg_tx;
                    }
                }
            }
        });

        // Spawn SIGWINCH handler (Unix only)
        #[cfg(unix)]
        let sigwinch_handle = {
            let sig_cancel = cancel.clone();
            let sig_msg_tx = msg_tx.clone();
            tokio::spawn(async move {
                use tokio::signal::unix::{SignalKind, signal};
                let mut stream = match signal(SignalKind::window_change()) {
                    Ok(s) => s,
                    Err(_) => return,
                };
                loop {
                    tokio::select! {
                        _ = sig_cancel.cancelled() => break,
                        _ = stream.recv() => {
                            if let Ok((w, h)) = terminal::size() {
                                let _ = sig_msg_tx.send(Msg::WindowSize { width: w, height: h });
                            }
                        }
                    }
                }
            })
        };

        // Drop the internal sender — channel stays open if ProgramHandle holds a clone
        drop(msg_tx);

        // Sequence tracking
        let mut sequence_queue: Vec<CmdInner> = Vec::new();

        // Render ticker for flushing
        let mut flush_interval = tokio::time::interval(std::time::Duration::from_micros(
            1_000_000 / self.fps as u64,
        ));

        let mut result: Result<(), ProgramError> = Ok(());

        // Main event loop
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                maybe_msg = msg_rx.recv() => {
                    match maybe_msg {
                        Some(msg) => {
                            // Handle custom internal messages (exec, raw)
                            let msg = match msg {
                                Msg::Custom(any) if any.is::<ExecRequest>() => {
                                    let exec_req = *any.downcast::<ExecRequest>().unwrap();
                                    // Release terminal
                                    let _ = terminal::disable_raw_mode();
                                    if current_alt_screen {
                                        let _ = execute!(stdout, terminal::LeaveAlternateScreen);
                                    }
                                    let _ = execute!(stdout, cursor::Show);

                                    // Execute subprocess
                                    let status = std::process::Command::new(&exec_req.program)
                                        .args(&exec_req.args)
                                        .status();

                                    // Restore terminal
                                    let _ = terminal::enable_raw_mode();
                                    if current_alt_screen {
                                        let _ = execute!(stdout, terminal::EnterAlternateScreen);
                                    }
                                    if current_bracketed_paste {
                                        let _ = execute!(stdout, event::EnableBracketedPaste);
                                    }
                                    if current_kitty_keyboard {
                                        let _ = execute!(
                                            stdout,
                                            event::PushKeyboardEnhancementFlags(
                                                event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                                            )
                                        );
                                    }
                                    screen.clear();
                                    render_dirty = true;

                                    // Deliver callback result
                                    let callback_msg = (exec_req.callback)(status);
                                    if let Some(cmd_inner) = self.model.update(callback_msg) {
                                        let _ = cmd_tx.send(cmd_inner);
                                    }
                                    continue;
                                }
                                Msg::Custom(any) if any.is::<RawSequence>() => {
                                    let raw_seq = *any.downcast::<RawSequence>().unwrap();
                                    let _ = stdout.write_all(raw_seq.0.as_bytes());
                                    let _ = stdout.flush();
                                    continue;
                                }
                                other => other,
                            };

                            // Handle suspend (Unix SIGTSTP)
                            if matches!(msg, Msg::Suspend) {
                                #[cfg(unix)]
                                {
                                    // Release terminal
                                    let _ = terminal::disable_raw_mode();
                                    if current_alt_screen {
                                        let _ = execute!(stdout, terminal::LeaveAlternateScreen);
                                    }
                                    let _ = execute!(stdout, cursor::Show);

                                    // Send SIGTSTP to process group
                                    let _ = nix::sys::signal::kill(
                                        nix::unistd::Pid::from_raw(0),
                                        nix::sys::signal::Signal::SIGTSTP,
                                    );

                                    // When we get here, SIGCONT was received
                                    // Restore terminal
                                    let _ = terminal::enable_raw_mode();
                                    if current_alt_screen {
                                        let _ = execute!(stdout, terminal::EnterAlternateScreen);
                                    }
                                    if current_bracketed_paste {
                                        let _ = execute!(stdout, event::EnableBracketedPaste);
                                    }
                                    if current_kitty_keyboard {
                                        let _ = execute!(
                                            stdout,
                                            event::PushKeyboardEnhancementFlags(
                                                event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                                            )
                                        );
                                    }
                                    screen.clear();
                                    render_dirty = true;

                                    // Send Resume to model
                                    if let Some(cmd_inner) = self.model.update(Msg::Resume) {
                                        let _ = cmd_tx.send(cmd_inner);
                                    }
                                }
                                continue;
                            }

                            let should_quit = matches!(msg, Msg::Quit | Msg::Interrupt);
                            let is_interrupt = matches!(msg, Msg::Interrupt);

                            // Apply message filter
                            let msg = if let Some(ref filter) = self.filter {
                                match filter(&self.model, msg) {
                                    Some(m) => m,
                                    None => continue, // Drop filtered message
                                }
                            } else {
                                msg
                            };

                            match msg {
                                Msg::Batch(cmds) => {
                                    for c in cmds.into_iter().flatten() {
                                        let _ = cmd_tx.send(c);
                                    }
                                }
                                Msg::Sequence(cmds) => {
                                    let mut valid: Vec<CmdInner> = cmds.into_iter().flatten().collect();
                                    if !valid.is_empty() {
                                        let first = valid.remove(0);
                                        let _ = cmd_tx.send(first);
                                        sequence_queue = valid;
                                    }
                                }
                                Msg::ClearScreen => {
                                    if !self.disable_renderer {
                                        screen.clear();
                                        render_dirty = true;
                                    }
                                }
                                Msg::WindowSize { width, height } => {
                                    screen.resize(width, height);
                                    if let Some(cmd_inner) = self.model.update(Msg::WindowSize { width, height }) {
                                        let _ = cmd_tx.send(cmd_inner);
                                    }
                                    if !self.disable_renderer {
                                        let view = self.model.view();
                                        apply_view_state(
                                            &mut stdout,
                                            &view,
                                            &mut current_alt_screen,
                                            &mut current_mouse_mode,
                                            &mut current_report_focus,
                                        )?;
                                        render_view(&mut screen, &view);
                                        screen.render(&mut stdout)?;
                                        render_cursor(&mut stdout, &view)?;
                                        last_view_hash = Some(hash_string(&view.content));
                                        render_dirty = false;
                                    }
                                }
                                msg => {
                                    if let Some(cmd_inner) = self.model.update(msg) {
                                        let _ = cmd_tx.send(cmd_inner);
                                    }

                                    if !sequence_queue.is_empty() {
                                        let next = sequence_queue.remove(0);
                                        let _ = cmd_tx.send(next);
                                    }

                                    if !self.disable_renderer {
                                        let view = self.model.view();

                                        apply_view_state(
                                            &mut stdout,
                                            &view,
                                            &mut current_alt_screen,
                                            &mut current_mouse_mode,
                                            &mut current_report_focus,
                                        )?;

                                        let uses_regions = !view.regions.is_empty();
                                        let view_hash = hash_string(&view.content);
                                        let content_changed = if uses_regions {
                                            // Always re-draw when using regions (diff engine deduplicates)
                                            true
                                        } else {
                                            last_view_hash != Some(view_hash)
                                        };
                                        if content_changed {
                                            render_view(&mut screen, &view);
                                            render_dirty = true;
                                            last_view_hash = Some(view_hash);
                                        }

                                        if render_dirty {
                                            screen.render(&mut stdout)?;
                                            render_cursor(&mut stdout, &view)?;
                                            render_dirty = false;
                                        }
                                    }
                                }
                            }

                            if should_quit {
                                if is_interrupt {
                                    result = Err(ProgramError::Interrupted);
                                }
                                break;
                            }
                        }
                        None => break,
                    }
                }
                _ = flush_interval.tick() => {
                    if !self.disable_renderer {
                        let _ = stdout.flush();
                    }
                }
            }
        }

        // Cancel all tasks
        cancel.cancel();

        // Update the guard
        let _guard = TerminalGuard {
            alt_screen: current_alt_screen,
            mouse_mode: current_mouse_mode,
            report_focus: current_report_focus,
            bracketed_paste: current_bracketed_paste,
            kitty_keyboard: current_kitty_keyboard,
            raw_mode: true,
        };
        std::mem::forget(guard);

        // Wait for tasks
        let _ = tokio::time::timeout(std::time::Duration::from_millis(100), input_handle).await;
        let _ = tokio::time::timeout(std::time::Duration::from_millis(100), cmd_handle).await;
        let _ = tokio::time::timeout(std::time::Duration::from_millis(100), render_handle).await;
        #[cfg(unix)]
        let _ = tokio::time::timeout(std::time::Duration::from_millis(100), sigwinch_handle).await;

        result?;
        Ok(self.model)
    }
}

/// Hash a string for cheap change detection (avoids cloning full content).
fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Execute a single command.
async fn execute_cmd(cmd: CmdInner) -> Msg {
    match cmd {
        CmdInner::Sync(f) => f(),
        CmdInner::Async(fut) => fut.await,
    }
}

/// Emergency terminal restoration (for panic recovery).
fn restore_terminal_emergency() {
    let mut stdout = io::stdout();
    let _ = execute!(stdout, cursor::Show);
    let _ = execute!(stdout, terminal::LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
}

/// Apply terminal state changes from a View.
fn apply_view_state(
    stdout: &mut io::Stdout,
    view: &View,
    current_alt_screen: &mut bool,
    current_mouse_mode: &mut MouseMode,
    current_report_focus: &mut bool,
) -> io::Result<()> {
    if view.alt_screen && !*current_alt_screen {
        execute!(stdout, terminal::EnterAlternateScreen)?;
        *current_alt_screen = true;
    } else if !view.alt_screen && *current_alt_screen {
        execute!(stdout, terminal::LeaveAlternateScreen)?;
        *current_alt_screen = false;
    }

    if view.mouse_mode != *current_mouse_mode {
        match *current_mouse_mode {
            MouseMode::CellMotion | MouseMode::AllMotion => {
                execute!(stdout, event::DisableMouseCapture)?;
            }
            MouseMode::None => {}
        }
        match view.mouse_mode {
            MouseMode::CellMotion | MouseMode::AllMotion => {
                execute!(stdout, event::EnableMouseCapture)?;
            }
            MouseMode::None => {}
        }
        *current_mouse_mode = view.mouse_mode;
    }

    if view.report_focus && !*current_report_focus {
        execute!(stdout, event::EnableFocusChange)?;
        *current_report_focus = true;
    } else if !view.report_focus && *current_report_focus {
        execute!(stdout, event::DisableFocusChange)?;
        *current_report_focus = false;
    }

    Ok(())
}

/// Render view content into the screen buffer.
/// Uses region-based drawing if regions are present, otherwise falls back to set_content.
fn render_view(screen: &mut ruse_ansi::Screen, view: &View) {
    if !view.regions.is_empty() {
        // Disable scroll optimization: CSI S/T scroll the entire terminal
        // width, which corrupts multi-region layouts where only one region
        // (e.g. chat) should scroll while another (e.g. sidebar) stays fixed.
        screen.set_scroll_optimize(false);
        screen.buffer_mut().clear();
        for (rect, content) in &view.regions {
            screen.draw_region(content, *rect);
        }
    } else {
        screen.set_scroll_optimize(true);
        screen.set_content(&view.content);
    }
}

/// Render cursor state.
fn render_cursor(stdout: &mut io::Stdout, view: &View) -> io::Result<()> {
    if let Some(ref cursor_view) = view.cursor
        && cursor_view.visible
    {
        let shape = match cursor_view.shape {
            CursorShape::Block => cursor::SetCursorStyle::SteadyBlock,
            CursorShape::Underline => cursor::SetCursorStyle::SteadyUnderScore,
            CursorShape::Bar => cursor::SetCursorStyle::SteadyBar,
        };
        queue!(
            stdout,
            cursor::MoveTo(cursor_view.x, cursor_view.y),
            shape,
            cursor::Show,
        )?;
        stdout.flush()?;
    }
    Ok(())
}
