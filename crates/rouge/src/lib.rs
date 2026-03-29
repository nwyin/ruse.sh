// Rouge: Rust TUI framework inspired by charm.sh
//
// Re-exports from all rouge crates for convenient single-crate usage.

// Core ANSI utilities
pub use rouge_ansi as ansi;

// Color profile detection
pub use rouge_colorprofile as colorprofile;

// Physics animation
pub use rouge_harmonica as harmonica;

// Styling, layout, borders, tables
pub use rouge_style as style;

// Elm architecture runtime
pub use rouge_runtime as runtime;

// UI components
pub use rouge_components as components;

// Markdown rendering
pub use rouge_glamour as glamour;

// Convenience re-exports of the most-used types
pub mod prelude {
    // Runtime
    pub use rouge_runtime::{
        Cmd, CmdInner, KeyCode, KeyEvent, Model, Modifiers, MouseButton, MouseEvent, MouseMode,
        Msg, Program, ProgramError, View,
    };
    pub use rouge_runtime::{
        batch, clear_screen, cmd, cmd_async, every, exec, println, quit, raw, read_clipboard,
        sequence, set_clipboard, tick, ProgramHandle,
    };

    // Styling
    pub use rouge_style::{
        Color, Position, Style, Border, UnderlineStyle,
        NORMAL_BORDER, ROUNDED_BORDER, BLOCK_BORDER, DOUBLE_BORDER, THICK_BORDER,
        HIDDEN_BORDER, ASCII_BORDER,
    };
    pub use rouge_style::{
        join_horizontal, join_vertical, place, place_horizontal, place_vertical,
        style_runes, style_ranges, StyleRange,
        blend_1d, blend_2d, light_dark, complete, darken, lighten, complementary, is_dark,
        Tree, Enumerator, Layer, Compositor, Whitespace,
    };

    // Components
    pub use rouge_components::{
        Binding, Cursor, CursorMode, FilePicker, Help, KeyMap, List, ListItem, Paginator,
        Progress, SimpleItem, Spinner, Stopwatch, TextArea, TextInput, Timer, Viewport,
    };
    pub use rouge_components::{
        dot_spinner, ellipsis_spinner, globe_spinner, line_spinner, mini_dot_spinner,
        moon_spinner, pulse_spinner, jump_spinner, points_spinner, monkey_spinner,
        meter_spinner, hamburger_spinner,
    };
}
