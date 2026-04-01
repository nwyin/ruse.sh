pub mod border;
pub mod color;
pub mod layer;
pub mod layout;
pub mod position;
pub mod render;
pub mod runes;
pub mod style;
pub mod table;
pub mod tree;
pub mod whitespace;

// Re-export core types at crate root
pub use border::{
    ASCII_BORDER, BLOCK_BORDER, Border, DOUBLE_BORDER, HIDDEN_BORDER, INNER_HALF_BLOCK_BORDER,
    MARKDOWN_BORDER, NO_BORDER, NORMAL_BORDER, OUTER_HALF_BLOCK_BORDER, ROUNDED_BORDER,
    THICK_BORDER,
};
pub use color::{
    Color, blend_1d, blend_2d, complementary, complete, darken, is_dark, light_dark, lighten,
};
pub use layer::{Compositor, Layer};
pub use layout::{
    height, join_horizontal, join_vertical, place, place_horizontal, place_vertical, width,
};
pub use position::Position;
pub use runes::{StyleRange, style_ranges, style_runes};
pub use style::{Props, Style, UnderlineStyle};
pub use table::{HEADER_ROW, Table};
pub use tree::{Enumerator, Tree};
pub use whitespace::Whitespace;
