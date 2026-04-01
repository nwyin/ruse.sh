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
        ProgramHandle, batch, clear_screen, cmd, cmd_async, every, exec, println, quit, raw,
        read_clipboard, sequence, set_clipboard, tick,
    };

    // Styling
    pub use ruse_style::{
        ASCII_BORDER, BLOCK_BORDER, Border, Color, DOUBLE_BORDER, HIDDEN_BORDER, NORMAL_BORDER,
        Position, ROUNDED_BORDER, Style, THICK_BORDER, UnderlineStyle,
    };
    pub use ruse_style::{
        Compositor, Enumerator, Layer, StyleRange, Tree, Whitespace, blend_1d, blend_2d,
        complementary, complete, darken, is_dark, join_horizontal, join_vertical, light_dark,
        lighten, place, place_horizontal, place_vertical, style_ranges, style_runes,
    };

    // Components
    pub use ruse_components::{
        Binding, Cursor, CursorMode, FilePicker, Help, KeyMap, List, ListItem, Paginator, Progress,
        SimpleItem, Spinner, Stopwatch, TextArea, TextInput, Timer, Viewport,
    };
    pub use ruse_components::{
        dot_spinner, ellipsis_spinner, globe_spinner, hamburger_spinner, jump_spinner,
        line_spinner, meter_spinner, mini_dot_spinner, monkey_spinner, moon_spinner,
        points_spinner, pulse_spinner,
    };
}
