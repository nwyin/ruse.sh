use std::any::Any;

use crate::cmd::Cmd;
use crate::key::KeyEvent;
use crate::mouse::MouseEvent;

#[non_exhaustive]
pub enum Msg {
    KeyPress(KeyEvent),
    KeyRelease(KeyEvent),
    MouseClick(MouseEvent),
    MouseRelease(MouseEvent),
    MouseWheel(MouseEvent),
    MouseMotion(MouseEvent),
    Paste(String),
    WindowSize { width: u16, height: u16 },
    Focus,
    Blur,
    Quit,
    Interrupt,
    Suspend,
    Resume,
    ClearScreen,
    PrintLine(String),
    Batch(Vec<Cmd>),
    Sequence(Vec<Cmd>),
    Custom(Box<dyn Any + Send>),
}

impl Msg {
    /// Wrap any `Send + 'static` value as a custom message.
    pub fn custom<T: Send + 'static>(val: T) -> Self {
        Msg::Custom(Box::new(val))
    }

    /// Try to downcast a reference to the inner custom value.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        match self {
            Msg::Custom(boxed) => boxed.downcast_ref::<T>(),
            _ => None,
        }
    }

    /// Try to downcast the inner custom value, consuming the message.
    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        match self {
            Msg::Custom(boxed) => match boxed.downcast::<T>() {
                Ok(val) => Ok(*val),
                Err(boxed) => Err(Msg::Custom(boxed)),
            },
            other => Err(other),
        }
    }
}

// Msg cannot derive Debug because of Box<dyn Any + Send>, so implement it manually.
impl std::fmt::Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Msg::KeyPress(e) => f.debug_tuple("KeyPress").field(e).finish(),
            Msg::KeyRelease(e) => f.debug_tuple("KeyRelease").field(e).finish(),
            Msg::MouseClick(e) => f.debug_tuple("MouseClick").field(e).finish(),
            Msg::MouseRelease(e) => f.debug_tuple("MouseRelease").field(e).finish(),
            Msg::MouseWheel(e) => f.debug_tuple("MouseWheel").field(e).finish(),
            Msg::MouseMotion(e) => f.debug_tuple("MouseMotion").field(e).finish(),
            Msg::Paste(s) => f.debug_tuple("Paste").field(s).finish(),
            Msg::WindowSize { width, height } => f
                .debug_struct("WindowSize")
                .field("width", width)
                .field("height", height)
                .finish(),
            Msg::Focus => write!(f, "Focus"),
            Msg::Blur => write!(f, "Blur"),
            Msg::Quit => write!(f, "Quit"),
            Msg::Interrupt => write!(f, "Interrupt"),
            Msg::Suspend => write!(f, "Suspend"),
            Msg::Resume => write!(f, "Resume"),
            Msg::ClearScreen => write!(f, "ClearScreen"),
            Msg::PrintLine(s) => f.debug_tuple("PrintLine").field(s).finish(),
            Msg::Batch(cmds) => f.debug_tuple("Batch").field(&cmds.len()).finish(),
            Msg::Sequence(cmds) => f.debug_tuple("Sequence").field(&cmds.len()).finish(),
            Msg::Custom(_) => write!(f, "Custom(...)"),
        }
    }
}
