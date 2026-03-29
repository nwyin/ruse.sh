use crossterm::event::{
    Event, KeyEvent as CtKeyEvent, KeyEventKind, MouseButton as CtMouseButton,
    MouseEvent as CtMouseEvent, MouseEventKind,
};

use crate::key::{KeyCode, KeyEvent, Modifiers};
use crate::mouse::{MouseButton, MouseEvent};
use crate::msg::Msg;

/// Translate a crossterm `Event` into our `Msg` type.
pub fn translate_event(event: Event) -> Option<Msg> {
    match event {
        Event::Key(key_event) => translate_key(key_event),
        Event::Mouse(mouse_event) => translate_mouse(mouse_event),
        Event::Resize(w, h) => Some(Msg::WindowSize {
            width: w,
            height: h,
        }),
        Event::Paste(s) => Some(Msg::Paste(s)),
        Event::FocusGained => Some(Msg::Focus),
        Event::FocusLost => Some(Msg::Blur),
    }
}

fn translate_key(ct: CtKeyEvent) -> Option<Msg> {
    let code: KeyCode = ct.code.into();
    let modifiers: Modifiers = ct.modifiers.into();

    let key_event = KeyEvent {
        code,
        modifiers,
        is_repeat: ct.kind == KeyEventKind::Repeat,
    };

    match ct.kind {
        KeyEventKind::Press | KeyEventKind::Repeat => Some(Msg::KeyPress(key_event)),
        KeyEventKind::Release => Some(Msg::KeyRelease(key_event)),
    }
}

fn translate_mouse(ct: CtMouseEvent) -> Option<Msg> {
    let modifiers: Modifiers = ct.modifiers.into();

    let make_event = |button: MouseButton| MouseEvent {
        x: ct.column,
        y: ct.row,
        button,
        modifiers,
    };

    match ct.kind {
        MouseEventKind::Down(btn) => {
            let button = translate_mouse_button(btn);
            Some(Msg::MouseClick(make_event(button)))
        }
        MouseEventKind::Up(btn) => {
            let button = translate_mouse_button(btn);
            Some(Msg::MouseRelease(make_event(button)))
        }
        MouseEventKind::Drag(btn) => {
            let button = translate_mouse_button(btn);
            Some(Msg::MouseMotion(make_event(button)))
        }
        MouseEventKind::Moved => Some(Msg::MouseMotion(make_event(MouseButton::None))),
        MouseEventKind::ScrollUp => Some(Msg::MouseWheel(make_event(MouseButton::WheelUp))),
        MouseEventKind::ScrollDown => Some(Msg::MouseWheel(make_event(MouseButton::WheelDown))),
        MouseEventKind::ScrollLeft => Some(Msg::MouseWheel(make_event(MouseButton::WheelLeft))),
        MouseEventKind::ScrollRight => Some(Msg::MouseWheel(make_event(MouseButton::WheelRight))),
    }
}

fn translate_mouse_button(btn: CtMouseButton) -> MouseButton {
    match btn {
        CtMouseButton::Left => MouseButton::Left,
        CtMouseButton::Right => MouseButton::Right,
        CtMouseButton::Middle => MouseButton::Middle,
    }
}
