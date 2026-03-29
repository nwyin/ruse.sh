use crate::cmd::Cmd;
use crate::msg::Msg;
use crate::view::View;

/// The Model trait defines the Elm-architecture interface for a TUI program.
///
/// Implementors provide:
/// - `init()`: optional initial command to run at startup
/// - `update(msg)`: handle a message, potentially returning a command
/// - `view()`: render the current state as a `View`
pub trait Model: Send + 'static {
    /// Called once when the program starts. Return a command to perform
    /// an initial action, or `None` to do nothing.
    fn init(&mut self) -> Cmd {
        None
    }

    /// Called when a message is received. Update the model state and
    /// optionally return a command.
    fn update(&mut self, msg: Msg) -> Cmd;

    /// Render the current model state as a `View`.
    fn view(&self) -> View;
}
