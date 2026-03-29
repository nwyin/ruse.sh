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
    pub use rouge_runtime::{batch, clear_screen, cmd, cmd_async, println, quit, sequence, tick};

    // Styling
    pub use rouge_style::{
        Color, Position, Style, Border,
        NORMAL_BORDER, ROUNDED_BORDER, BLOCK_BORDER, DOUBLE_BORDER, THICK_BORDER,
        HIDDEN_BORDER, ASCII_BORDER,
    };
    pub use rouge_style::{
        join_horizontal, join_vertical, place, place_horizontal, place_vertical,
    };

    // Components
    pub use rouge_components::{
        Binding, Cursor, CursorMode, FilePicker, Help, KeyMap, List, ListItem, Paginator,
        Progress, SimpleItem, Spinner, Stopwatch, TextArea, TextInput, Timer, Viewport,
    };
    pub use rouge_components::{
        dot_spinner, ellipsis_spinner, globe_spinner, line_spinner, mini_dot_spinner,
        moon_spinner, pulse_spinner,
    };
}
