use crate::key::Modifiers;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseEvent {
    pub x: u16,
    pub y: u16,
    pub button: MouseButton,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
    None,
}
