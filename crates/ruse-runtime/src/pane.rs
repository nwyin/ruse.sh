use std::any::Any;

use crate::cmd::Cmd;
use crate::msg::Msg;

/// A composable UI pane — the building block for composite layouts.
///
/// Formalizes the component convention already used by TextInput, List,
/// Viewport, etc. into a trait that [`Scene`](crate::scene::Scene) can
/// dispatch to.
///
/// Panes receive messages by reference (`&Msg`) so that a Scene can
/// broadcast a single message to multiple panes without cloning.
pub trait Pane: Send + 'static {
    /// Handle a message. Called for every message routed to this pane.
    fn update(&mut self, msg: &Msg) -> Cmd;

    /// Render the pane's current state as a styled string.
    fn view(&self) -> String;

    /// Called once when the pane is first added to a Scene.
    fn init(&mut self) -> Cmd {
        None
    }

    /// Called when this pane gains input focus.
    fn focus(&mut self) -> Cmd {
        None
    }

    /// Called when this pane loses input focus.
    fn blur(&mut self) {}

    /// Whether this pane currently considers itself focused.
    fn focused(&self) -> bool {
        false
    }
}

/// Internal trait that extends Pane with Any downcasting.
///
/// The blanket impl means users only need to implement `Pane`.
/// Scene stores `Box<dyn PaneAny>` and uses `as_any()` for
/// type-safe access via `scene.pane_as::<T>(id)`.
pub(crate) trait PaneAny: Pane {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Pane + 'static> PaneAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Tags a Cmd result with the originating pane's ID so Scene can route
/// it back to the correct pane.
pub(crate) struct PaneMsg {
    pub pane_id: String,
    pub inner: Msg,
}
