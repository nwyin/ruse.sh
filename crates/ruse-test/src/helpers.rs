use ruse_runtime::{KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent, Msg};

/// Build a `KeyPress` message from a `KeyCode` with no modifiers.
pub fn key(code: KeyCode) -> Msg {
    Msg::KeyPress(KeyEvent {
        code,
        modifiers: Modifiers::empty(),
        is_repeat: false,
    })
}

/// Build a `KeyPress` message for a character key.
pub fn char_key(ch: char) -> Msg {
    key(KeyCode::Char(ch))
}

/// Build a `KeyPress` message with modifiers.
pub fn key_with_mods(code: KeyCode, mods: Modifiers) -> Msg {
    Msg::KeyPress(KeyEvent {
        code,
        modifiers: mods,
        is_repeat: false,
    })
}

/// Convert a string into a `Vec<Msg>` of `KeyPress` events, one per character.
pub fn type_string(s: &str) -> Vec<Msg> {
    s.chars().map(|ch| char_key(ch)).collect()
}

/// Build a `WindowSize` message.
pub fn window_size(width: u16, height: u16) -> Msg {
    Msg::WindowSize { width, height }
}

/// Build a `MouseClick` message.
pub fn mouse_click(button: MouseButton, x: u16, y: u16) -> Msg {
    Msg::MouseClick(MouseEvent {
        button,
        x,
        y,
        modifiers: Modifiers::empty(),
    })
}
