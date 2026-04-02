use std::sync::{Arc, Mutex};
use std::time::Duration;

use ruse_runtime::cmd::Cmd;
use ruse_runtime::error::ProgramError;
use ruse_runtime::view::View;
use ruse_runtime::{KeyCode, Model, Modifiers, Msg, Program, ProgramHandle};
use tokio::task::JoinHandle;

use crate::helpers;

/// Private wrapper that captures view output after every `init()` and `update()`.
struct SpyModel<M: Model> {
    inner: M,
    view_log: Arc<Mutex<Vec<String>>>,
}

impl<M: Model> SpyModel<M> {
    fn capture_view(&self) {
        let view = self.inner.view();
        let mut log = self.view_log.lock().expect("view_log lock poisoned");
        log.push(view.content);
    }

    fn into_inner(self) -> M {
        self.inner
    }
}

impl<M: Model> Model for SpyModel<M> {
    fn init(&mut self) -> Cmd {
        let cmd = self.inner.init();
        self.capture_view();
        cmd
    }

    fn update(&mut self, msg: Msg) -> Cmd {
        let cmd = self.inner.update(msg);
        self.capture_view();
        cmd
    }

    fn view(&self) -> View {
        self.inner.view()
    }
}

/// Async test harness wrapping a running `Program` in headless mode.
///
/// Uses `ProgramHandle` to inject messages and a `SpyModel` wrapper
/// to capture view output after every update.
///
/// # Example
/// ```ignore
/// #[tokio::test]
/// async fn test_counter() {
///     let t = ProgramTest::new(Counter::new()).await;
///     t.send_key(KeyCode::Up).unwrap();
///     t.settle().await;
///     assert!(t.latest_view().unwrap().contains("Count: 1"));
///     let model = t.quit().await.unwrap();
///     assert_eq!(model.count, 1);
/// }
/// ```
pub struct ProgramTest<M: Model> {
    handle: ProgramHandle,
    join: JoinHandle<Result<SpyModel<M>, ProgramError>>,
    view_log: Arc<Mutex<Vec<String>>>,
}

impl<M: Model + 'static> ProgramTest<M> {
    /// Create and start a headless program test with default size (80x24).
    pub async fn new(model: M) -> Self {
        Self::with_size(model, 80, 24).await
    }

    /// Create and start a headless program test with a specific terminal size.
    pub async fn with_size(model: M, width: u16, height: u16) -> Self {
        let view_log = Arc::new(Mutex::new(Vec::new()));

        let spy = SpyModel {
            inner: model,
            view_log: view_log.clone(),
        };

        let (handle, fut) = Program::new(spy)
            .headless_with_size(width, height)
            .run_with_handle();

        let join = tokio::spawn(fut);

        // Let the runtime start and process init + WindowSize
        tokio::time::sleep(Duration::from_millis(10)).await;

        Self {
            handle,
            join,
            view_log,
        }
    }

    /// Send a message to the running program.
    pub fn send(&self, msg: Msg) -> Result<(), &'static str> {
        self.handle.send(msg)
    }

    /// Send a `KeyPress` for a character.
    pub fn send_char(&self, ch: char) -> Result<(), &'static str> {
        self.send(helpers::char_key(ch))
    }

    /// Send a `KeyPress` for a `KeyCode`.
    pub fn send_key(&self, code: KeyCode) -> Result<(), &'static str> {
        self.send(helpers::key(code))
    }

    /// Send a `KeyPress` with modifiers.
    pub fn send_key_with_mods(&self, code: KeyCode, mods: Modifiers) -> Result<(), &'static str> {
        self.send(helpers::key_with_mods(code, mods))
    }

    /// Type a string as individual `KeyPress` messages.
    pub fn type_string(&self, s: &str) -> Result<(), &'static str> {
        for msg in helpers::type_string(s) {
            self.send(msg)?;
        }
        Ok(())
    }

    /// Wait briefly for the program to process pending messages.
    pub async fn settle(&self) {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    /// Wait for the view output to match a condition, polling at intervals.
    /// Panics on timeout.
    pub async fn wait_for<F>(&self, condition: F, timeout: Duration) -> String
    where
        F: Fn(&str) -> bool,
    {
        let deadline = tokio::time::Instant::now() + timeout;
        let poll_interval = Duration::from_millis(25);

        loop {
            {
                let log = self.view_log.lock().expect("view_log lock poisoned");
                if let Some(last) = log.last() {
                    if condition(last) {
                        return last.clone();
                    }
                }
            }

            if tokio::time::Instant::now() >= deadline {
                let log = self.view_log.lock().expect("view_log lock poisoned");
                let last = log
                    .last()
                    .map(|s| s.as_str())
                    .unwrap_or("<no views captured>");
                panic!(
                    "wait_for timed out after {:?}. Last view:\n{}",
                    timeout, last
                );
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Get a snapshot of all captured views (cloned).
    pub fn view_log(&self) -> Vec<String> {
        self.view_log
            .lock()
            .expect("view_log lock poisoned")
            .clone()
    }

    /// Get the most recent view content.
    pub fn latest_view(&self) -> Option<String> {
        self.view_log
            .lock()
            .expect("view_log lock poisoned")
            .last()
            .cloned()
    }

    /// Request graceful quit, wait for the program to finish, and return
    /// the final model state. Times out after 5 seconds.
    pub async fn quit(self) -> Result<M, ProgramError> {
        self.handle.quit();
        match tokio::time::timeout(Duration::from_secs(5), self.join).await {
            Ok(Ok(result)) => result.map(|spy| spy.into_inner()),
            Ok(Err(_join_err)) => Err(ProgramError::Killed),
            Err(_timeout) => {
                self.handle.kill();
                Err(ProgramError::Killed)
            }
        }
    }

    /// Force-kill the program immediately.
    pub async fn kill(self) -> Result<M, ProgramError> {
        self.handle.kill();
        match tokio::time::timeout(Duration::from_secs(2), self.join).await {
            Ok(Ok(result)) => result.map(|spy| spy.into_inner()),
            _ => Err(ProgramError::Killed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruse_runtime::cmd;
    use ruse_runtime::view::View;

    struct Counter {
        count: i32,
    }

    impl Model for Counter {
        fn update(&mut self, msg: Msg) -> Cmd {
            match msg {
                Msg::KeyPress(ref e) => match e.code {
                    KeyCode::Up => self.count += 1,
                    KeyCode::Down => self.count -= 1,
                    KeyCode::Char('q') => return cmd::quit(),
                    _ => {}
                },
                _ => {}
            }
            None
        }

        fn view(&self) -> View {
            View::new(format!("Count: {}", self.count))
        }
    }

    #[tokio::test]
    async fn test_program_send_and_quit() {
        let t = ProgramTest::new(Counter { count: 0 }).await;
        t.send_key(KeyCode::Up).unwrap();
        t.send_key(KeyCode::Up).unwrap();
        t.settle().await;

        let model = t.quit().await.unwrap();
        assert_eq!(model.count, 2);
    }

    #[tokio::test]
    async fn test_program_view_log() {
        let t = ProgramTest::new(Counter { count: 0 }).await;
        t.send_key(KeyCode::Up).unwrap();
        t.settle().await;

        let views = t.view_log();
        // Should have at least init + WindowSize + KeyPress views
        assert!(views.len() >= 2);

        let model = t.quit().await.unwrap();
        assert_eq!(model.count, 1);
    }

    #[tokio::test]
    async fn test_program_wait_for() {
        let t = ProgramTest::new(Counter { count: 0 }).await;
        t.send_key(KeyCode::Up).unwrap();
        t.send_key(KeyCode::Up).unwrap();
        t.send_key(KeyCode::Up).unwrap();

        let view = t
            .wait_for(|v| v.contains("Count: 3"), Duration::from_secs(1))
            .await;
        assert!(view.contains("Count: 3"));

        t.quit().await.unwrap();
    }

    #[tokio::test]
    async fn test_program_latest_view() {
        let t = ProgramTest::new(Counter { count: 0 }).await;
        t.send_key(KeyCode::Up).unwrap();
        t.settle().await;

        let latest = t.latest_view().unwrap();
        assert!(latest.contains("Count: 1"));

        t.quit().await.unwrap();
    }

    #[tokio::test]
    async fn test_program_quit_via_model() {
        let t = ProgramTest::new(Counter { count: 0 }).await;
        t.send_key(KeyCode::Up).unwrap();
        t.send_char('q').unwrap();

        let model = t.quit().await.unwrap();
        assert_eq!(model.count, 1);
    }

    #[tokio::test]
    async fn test_program_custom_size() {
        let t = ProgramTest::with_size(Counter { count: 0 }, 120, 40).await;
        t.send_key(KeyCode::Up).unwrap();
        t.settle().await;
        let model = t.quit().await.unwrap();
        assert_eq!(model.count, 1);
    }

    #[tokio::test]
    async fn test_program_kill() {
        let t = ProgramTest::new(Counter { count: 0 }).await;
        t.send_key(KeyCode::Up).unwrap();
        t.settle().await;
        // Kill should not hang
        let _ = t.kill().await;
    }
}
