use crossterm::event::{KeyCode as CtKeyCode, KeyModifiers as CtKeyModifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: Modifiers,
    pub is_repeat: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Enter,
    Backspace,
    Tab,
    BackTab,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    F(u8),
    Null,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Modifiers: u8 {
        const SHIFT = 0b0001;
        const ALT   = 0b0010;
        const CTRL  = 0b0100;
        const SUPER = 0b1000;
    }
}

impl From<CtKeyCode> for KeyCode {
    fn from(code: CtKeyCode) -> Self {
        match code {
            CtKeyCode::Char(c) => KeyCode::Char(c),
            CtKeyCode::Enter => KeyCode::Enter,
            CtKeyCode::Backspace => KeyCode::Backspace,
            CtKeyCode::Tab => KeyCode::Tab,
            CtKeyCode::BackTab => KeyCode::BackTab,
            CtKeyCode::Esc => KeyCode::Escape,
            CtKeyCode::Up => KeyCode::Up,
            CtKeyCode::Down => KeyCode::Down,
            CtKeyCode::Left => KeyCode::Left,
            CtKeyCode::Right => KeyCode::Right,
            CtKeyCode::Home => KeyCode::Home,
            CtKeyCode::End => KeyCode::End,
            CtKeyCode::PageUp => KeyCode::PageUp,
            CtKeyCode::PageDown => KeyCode::PageDown,
            CtKeyCode::Insert => KeyCode::Insert,
            CtKeyCode::Delete => KeyCode::Delete,
            CtKeyCode::F(n) => KeyCode::F(n),
            CtKeyCode::Null => KeyCode::Null,
            _ => KeyCode::Null,
        }
    }
}

impl From<CtKeyModifiers> for Modifiers {
    fn from(mods: CtKeyModifiers) -> Self {
        let mut result = Modifiers::empty();
        if mods.contains(CtKeyModifiers::SHIFT) {
            result |= Modifiers::SHIFT;
        }
        if mods.contains(CtKeyModifiers::ALT) {
            result |= Modifiers::ALT;
        }
        if mods.contains(CtKeyModifiers::CONTROL) {
            result |= Modifiers::CTRL;
        }
        if mods.contains(CtKeyModifiers::SUPER) {
            result |= Modifiers::SUPER;
        }
        result
    }
}
