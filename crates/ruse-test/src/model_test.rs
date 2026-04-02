use ruse_runtime::cmd::CmdInner;
use ruse_runtime::view::View;
use ruse_runtime::{Cmd, KeyCode, Model, Modifiers, Msg};

use crate::helpers;

/// Synchronous test harness for a `Model`.
///
/// Drives a model directly by calling `init()`, `update()`, and `view()`
/// without any runtime, terminal, or async. Commands returned by the model
/// are accumulated and can be drained or executed.
///
/// # Example
/// ```ignore
/// let mut t = ModelTest::new(MyModel::new());
/// t.init().send_key(KeyCode::Up).send_key(KeyCode::Up);
/// assert_eq!(t.model().count, 2);
/// ```
pub struct ModelTest<M: Model> {
    model: M,
    width: u16,
    height: u16,
    pending_cmds: Vec<CmdInner>,
}

impl<M: Model> ModelTest<M> {
    /// Wrap a model for testing. Does NOT call `init()`.
    pub fn new(model: M) -> Self {
        Self {
            model,
            width: 80,
            height: 24,
            pending_cmds: Vec::new(),
        }
    }

    /// Call `model.init()` and send an initial `WindowSize` message.
    pub fn init(&mut self) -> &mut Self {
        let cmd = self.model.init();
        self.push_cmd(cmd);
        let cmd = self.model.update(Msg::WindowSize {
            width: self.width,
            height: self.height,
        });
        self.push_cmd(cmd);
        self
    }

    /// Set the terminal size and send a `WindowSize` message.
    pub fn resize(&mut self, width: u16, height: u16) -> &mut Self {
        self.width = width;
        self.height = height;
        self.send(Msg::WindowSize { width, height })
    }

    /// Send a message to the model.
    pub fn send(&mut self, msg: Msg) -> &mut Self {
        let cmd = self.model.update(msg);
        self.push_cmd(cmd);
        self
    }

    /// Send a `KeyPress` for a character.
    pub fn send_char(&mut self, ch: char) -> &mut Self {
        self.send(helpers::char_key(ch))
    }

    /// Send a `KeyPress` for a `KeyCode`.
    pub fn send_key(&mut self, code: KeyCode) -> &mut Self {
        self.send(helpers::key(code))
    }

    /// Send a `KeyPress` with modifiers.
    pub fn send_key_with_mods(&mut self, code: KeyCode, mods: Modifiers) -> &mut Self {
        self.send(helpers::key_with_mods(code, mods))
    }

    /// Type a string as individual `KeyPress` messages.
    pub fn type_string(&mut self, s: &str) -> &mut Self {
        for msg in helpers::type_string(s) {
            self.send(msg);
        }
        self
    }

    /// Send a `Paste` message.
    pub fn paste(&mut self, s: &str) -> &mut Self {
        self.send(Msg::Paste(s.to_string()))
    }

    /// Get the current `View` from `model.view()`.
    pub fn view(&self) -> View {
        self.model.view()
    }

    /// Get the view content string (raw, with ANSI escapes).
    pub fn view_content(&self) -> String {
        self.model.view().content
    }

    /// Get the view content with ANSI escapes stripped (plain text).
    pub fn view_plain(&self) -> String {
        ruse_ansi::strip::strip_ansi(&self.model.view().content)
    }

    /// Get an immutable reference to the model.
    pub fn model(&self) -> &M {
        &self.model
    }

    /// Get a mutable reference to the model.
    pub fn model_mut(&mut self) -> &mut M {
        &mut self.model
    }

    /// Consume the harness, returning the model.
    pub fn into_model(self) -> M {
        self.model
    }

    /// Drain all accumulated commands (from `init`/`update` calls).
    pub fn drain_cmds(&mut self) -> Vec<CmdInner> {
        std::mem::take(&mut self.pending_cmds)
    }

    /// Execute all accumulated sync commands, feeding their result messages
    /// back into `model.update()`. Async commands cannot be executed without
    /// a runtime and are returned separately.
    ///
    /// Handles `Msg::Batch` and `Msg::Sequence` by unpacking inner commands
    /// and queuing them for execution, mirroring the runtime's behavior.
    pub fn execute_sync_cmds(&mut self) -> Vec<CmdInner> {
        let mut async_cmds = Vec::new();
        let mut queue = std::mem::take(&mut self.pending_cmds);

        while let Some(cmd_inner) = queue.pop() {
            match cmd_inner {
                CmdInner::Sync(f) => {
                    let msg = f();
                    match msg {
                        Msg::Batch(cmds) => {
                            for c in cmds.into_iter().flatten() {
                                queue.push(c);
                            }
                        }
                        Msg::Sequence(cmds) => {
                            // Sequence: execute one at a time. Queue them
                            // in reverse so the first is processed next.
                            let valid: Vec<CmdInner> = cmds.into_iter().flatten().collect();
                            for c in valid.into_iter().rev() {
                                queue.push(c);
                            }
                        }
                        msg => {
                            let cmd = self.model.update(msg);
                            if let Some(inner) = cmd {
                                queue.push(inner);
                            }
                        }
                    }
                }
                CmdInner::Async(fut) => {
                    async_cmds.push(CmdInner::Async(fut));
                }
            }
        }

        async_cmds
    }

    fn push_cmd(&mut self, cmd: Cmd) {
        if let Some(inner) = cmd {
            self.pending_cmds.push(inner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                    KeyCode::Char('r') => self.count = 0,
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

    #[test]
    fn test_new_does_not_call_init() {
        let t = ModelTest::new(Counter { count: 0 });
        assert_eq!(t.model().count, 0);
    }

    #[test]
    fn test_send_key() {
        let mut t = ModelTest::new(Counter { count: 0 });
        t.init();
        t.send_key(KeyCode::Up)
            .send_key(KeyCode::Up)
            .send_key(KeyCode::Up);
        assert_eq!(t.model().count, 3);
    }

    #[test]
    fn test_send_char() {
        let mut t = ModelTest::new(Counter { count: 5 });
        t.init();
        t.send_char('r');
        assert_eq!(t.model().count, 0);
    }

    #[test]
    fn test_send_key_down() {
        let mut t = ModelTest::new(Counter { count: 0 });
        t.init();
        t.send_key(KeyCode::Down).send_key(KeyCode::Down);
        assert_eq!(t.model().count, -2);
    }

    #[test]
    fn test_view_content() {
        let mut t = ModelTest::new(Counter { count: 0 });
        t.init();
        t.send_key(KeyCode::Up);
        assert_eq!(t.view_content(), "Count: 1");
    }

    #[test]
    fn test_view_plain() {
        let mut t = ModelTest::new(Counter { count: 0 });
        t.init();
        assert_eq!(t.view_plain(), "Count: 0");
    }

    #[test]
    fn test_resize() {
        let mut t = ModelTest::new(Counter { count: 0 });
        t.init();
        t.resize(120, 40);
        // Resize shouldn't affect counter, but model should still work
        t.send_key(KeyCode::Up);
        assert_eq!(t.model().count, 1);
    }

    #[test]
    fn test_chaining() {
        let mut t = ModelTest::new(Counter { count: 0 });
        t.init()
            .send_key(KeyCode::Up)
            .send_key(KeyCode::Up)
            .send_key(KeyCode::Down)
            .send_char('r')
            .send_key(KeyCode::Up);
        assert_eq!(t.model().count, 1);
    }

    #[test]
    fn test_into_model() {
        let mut t = ModelTest::new(Counter { count: 0 });
        t.init().send_key(KeyCode::Up).send_key(KeyCode::Up);
        let model = t.into_model();
        assert_eq!(model.count, 2);
    }

    #[test]
    fn test_paste() {
        // Counter ignores paste, but verify it doesn't panic
        let mut t = ModelTest::new(Counter { count: 0 });
        t.init().paste("hello");
        assert_eq!(t.model().count, 0);
    }

    #[test]
    fn test_snapshot() {
        let mut t = ModelTest::new(Counter { count: 0 });
        t.init().send_key(KeyCode::Up).send_key(KeyCode::Up);
        insta::assert_snapshot!(t.view_plain(), @"Count: 2");
    }
}
