use std::io::{self, Write};

use crossterm::{
    cursor, event,
    event::EventStream,
    execute, queue,
    terminal::{self, ClearType},
};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::cmd::CmdInner;
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
    raw_mode: bool,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();

        if self.report_focus {
            let _ = execute!(stdout, event::DisableFocusChange);
        }

        match self.mouse_mode {
            MouseMode::CellMotion => {
                let _ = execute!(stdout, event::DisableMouseCapture);
            }
            MouseMode::AllMotion => {
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

/// The main runtime for an Elm-architecture TUI program.
pub struct Program<M: Model> {
    model: M,
    fps: u32,
    alt_screen: bool,
    mouse_mode: MouseMode,
    report_focus: bool,
    disable_renderer: bool,
    #[allow(dead_code)]
    disable_signals: bool,
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
            disable_renderer: false,
            disable_signals: false,
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

    /// Run the program, blocking until it exits.
    /// Returns the final model state on success.
    pub async fn run(mut self) -> Result<M, ProgramError> {
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

        // Install the drop guard for cleanup
        let guard = TerminalGuard {
            alt_screen: current_alt_screen,
            mouse_mode: current_mouse_mode,
            report_focus: current_report_focus,
            raw_mode: true,
        };

        let cancel = CancellationToken::new();
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<Msg>();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<CmdInner>();

        // Call model.init() and submit any returned command
        if let Some(cmd_inner) = self.model.init() {
            let _ = cmd_tx.send(cmd_inner);
        }

        // Render initial view
        if !self.disable_renderer {
            let view = self.model.view();
            render_view(&mut stdout, &view, &mut current_alt_screen, &mut current_mouse_mode, &mut current_report_focus)?;
            stdout.flush()?;
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

        // Spawn render ticker task (flushes stdout at the target FPS)
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
                        // Send a no-op signal to trigger a flush.
                        // We use a custom message type that will be ignored in the update loop.
                        // Actually, we just flush stdout directly from the main loop on tick.
                        // Instead, use a lightweight "tick" approach: the main loop
                        // handles rendering after updates; the ticker just ensures periodic flushes.
                        // We'll send nothing here; instead the main loop will flush on its own interval.
                        let _ = &render_msg_tx; // keep alive
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

        // Drop the last sender clone so the channel closes when all tasks are done
        drop(msg_tx);

        // Sequence tracking: remaining commands to execute in order
        let mut sequence_queue: Vec<CmdInner> = Vec::new();

        // Render ticker for flushing
        let mut flush_interval =
            tokio::time::interval(std::time::Duration::from_micros(1_000_000 / self.fps as u64));

        let mut result: Result<(), ProgramError> = Ok(());

        // Main event loop
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                maybe_msg = msg_rx.recv() => {
                    match maybe_msg {
                        Some(msg) => {
                            let should_quit = matches!(msg, Msg::Quit | Msg::Interrupt);
                            let is_interrupt = matches!(msg, Msg::Interrupt);

                            match msg {
                                Msg::Batch(cmds) => {
                                    // Send all commands concurrently
                                    for c in cmds.into_iter().flatten() {
                                        let _ = cmd_tx.send(c);
                                    }
                                }
                                Msg::Sequence(cmds) => {
                                    // Queue up sequence commands
                                    let mut valid: Vec<CmdInner> = cmds.into_iter().flatten().collect();
                                    if !valid.is_empty() {
                                        // Send first, queue rest
                                        let first = valid.remove(0);
                                        let _ = cmd_tx.send(first);
                                        sequence_queue = valid;
                                    }
                                }
                                Msg::ClearScreen => {
                                    if !self.disable_renderer {
                                        let _ = execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0));
                                    }
                                }
                                msg => {
                                    // Feed message to model
                                    if let Some(cmd_inner) = self.model.update(msg) {
                                        let _ = cmd_tx.send(cmd_inner);
                                    }

                                    // If we have a sequence queue waiting and no pending
                                    // sequence command, send the next one
                                    if !sequence_queue.is_empty() {
                                        let next = sequence_queue.remove(0);
                                        let _ = cmd_tx.send(next);
                                    }

                                    // Re-render
                                    if !self.disable_renderer {
                                        let view = self.model.view();
                                        let _ = render_view(
                                            &mut stdout,
                                            &view,
                                            &mut current_alt_screen,
                                            &mut current_mouse_mode,
                                            &mut current_report_focus,
                                        );
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
                        None => break, // All senders dropped
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

        // Update the guard with current terminal state before it drops
        // We need to make sure cleanup matches whatever state we're in
        let _guard = TerminalGuard {
            alt_screen: current_alt_screen,
            mouse_mode: current_mouse_mode,
            report_focus: current_report_focus,
            raw_mode: true,
        };
        // Drop the original guard without running its destructor since
        // we replaced it with an updated one
        std::mem::forget(guard);

        // Wait for tasks to complete (with a short timeout)
        let _ = tokio::time::timeout(std::time::Duration::from_millis(100), input_handle).await;
        let _ = tokio::time::timeout(std::time::Duration::from_millis(100), cmd_handle).await;
        let _ = tokio::time::timeout(std::time::Duration::from_millis(100), render_handle).await;
        #[cfg(unix)]
        let _ = tokio::time::timeout(std::time::Duration::from_millis(100), sigwinch_handle).await;

        // _guard drops here, restoring terminal

        result?;
        Ok(self.model)
    }
}

/// Execute a single command (sync or async) and return the resulting message.
async fn execute_cmd(cmd: CmdInner) -> Msg {
    match cmd {
        CmdInner::Sync(f) => f(),
        CmdInner::Async(fut) => fut.await,
    }
}

/// Render a view to stdout. Handles alt screen toggling, mouse mode changes,
/// focus reporting changes, and content output.
fn render_view(
    stdout: &mut io::Stdout,
    view: &View,
    current_alt_screen: &mut bool,
    current_mouse_mode: &mut MouseMode,
    current_report_focus: &mut bool,
) -> io::Result<()> {
    // Toggle alt screen if needed
    if view.alt_screen && !*current_alt_screen {
        execute!(stdout, terminal::EnterAlternateScreen)?;
        *current_alt_screen = true;
    } else if !view.alt_screen && *current_alt_screen {
        execute!(stdout, terminal::LeaveAlternateScreen)?;
        *current_alt_screen = false;
    }

    // Update mouse mode if changed
    if view.mouse_mode != *current_mouse_mode {
        // Disable current mode
        match *current_mouse_mode {
            MouseMode::CellMotion | MouseMode::AllMotion => {
                execute!(stdout, event::DisableMouseCapture)?;
            }
            MouseMode::None => {}
        }
        // Enable new mode
        match view.mouse_mode {
            MouseMode::CellMotion | MouseMode::AllMotion => {
                execute!(stdout, event::EnableMouseCapture)?;
            }
            MouseMode::None => {}
        }
        *current_mouse_mode = view.mouse_mode;
    }

    // Update focus reporting if changed
    if view.report_focus && !*current_report_focus {
        execute!(stdout, event::EnableFocusChange)?;
        *current_report_focus = true;
    } else if !view.report_focus && *current_report_focus {
        execute!(stdout, event::DisableFocusChange)?;
        *current_report_focus = false;
    }

    // Clear screen and write content
    queue!(
        stdout,
        cursor::Hide,
        cursor::MoveTo(0, 0),
        terminal::Clear(ClearType::All),
    )?;

    // Write the view content.
    // In raw mode, \n only moves down (LF) without returning to column 0.
    // Replace \n with \r\n so each line starts at column 0.
    let content = view.content.replace('\n', "\r\n");
    write!(stdout, "{}", content)?;

    // Handle cursor
    if let Some(ref cursor_view) = view.cursor
        && cursor_view.visible {
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
        }

    stdout.flush()?;
    Ok(())
}
