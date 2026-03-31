// Ruse: Rust TUI framework inspired by charm.sh
//
// Re-exports from all ruse crates for convenient single-crate usage.

// Core ANSI utilities
pub use ruse_ansi as ansi;

// Color profile detection
pub use ruse_colorprofile as colorprofile;

// Physics animation
pub use ruse_harmonica as harmonica;

// Styling, layout, borders, tables
pub use ruse_style as style;

// Elm architecture runtime
pub use ruse_runtime as runtime;

// UI components
pub use ruse_components as components;

// Markdown rendering
pub use ruse_glamour as glamour;

// Convenience re-exports of the most-used types
pub mod prelude {
    // Runtime
    pub use ruse_runtime::{
        Cmd, CmdInner, KeyCode, KeyEvent, Model, Modifiers, MouseButton, MouseEvent, MouseMode,
        Msg, Program, ProgramError, Rect, View,
    };
    pub use ruse_runtime::{
        batch, clear_screen, cmd, cmd_async, every, exec, println, quit, raw, read_clipboard,
        sequence, set_clipboard, tick, ProgramHandle,
    };

    // Styling
    pub use ruse_style::{
        Color, Position, Style, Border, UnderlineStyle,
        NORMAL_BORDER, ROUNDED_BORDER, BLOCK_BORDER, DOUBLE_BORDER, THICK_BORDER,
        HIDDEN_BORDER, ASCII_BORDER,
    };
    pub use ruse_style::{
        join_horizontal, join_vertical, place, place_horizontal, place_vertical,
        style_runes, style_ranges, StyleRange,
        blend_1d, blend_2d, light_dark, complete, darken, lighten, complementary, is_dark,
        Tree, Enumerator, Layer, Compositor, Whitespace,
    };

    // Components
    pub use ruse_components::{
        Binding, Cursor, CursorMode, FilePicker, Help, KeyMap, List, ListItem, Paginator,
        Progress, SimpleItem, Spinner, Stopwatch, TextArea, TextInput, Timer, Viewport,
    };
    pub use ruse_components::{
        dot_spinner, ellipsis_spinner, globe_spinner, line_spinner, mini_dot_spinner,
        moon_spinner, pulse_spinner, jump_spinner, points_spinner, monkey_spinner,
        meter_spinner, hamburger_spinner,
    };
}
