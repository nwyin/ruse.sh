pub mod cellbuf;
pub mod image_detect;
pub mod iterm2;
pub mod kitty;
pub mod mosaic;
pub mod osc;
pub mod pad;
pub mod palette;
pub mod parser;
pub mod sgr;
pub mod sixel;
pub mod strip;
pub mod truncate;
mod util;
pub mod vt;
pub mod width;
pub mod wrap;

pub use cellbuf::{
    AttrMask, Buffer, Cell, CellStyle, Link, Position, Rect, Screen, ScreenOptions, UnderlineStyle,
};
pub use image_detect::{ImageProtocol, detect_image_protocol};
pub use iterm2::encode_iterm2_image;
pub use kitty::{KittyOptions, encode_image};
pub use mosaic::render_mosaic;
pub use osc::{
    hyperlink_close, hyperlink_open, notify, request_clipboard, set_clipboard, set_window_title,
};
pub use pad::{pad, pad_left, pad_right};
pub use palette::Palette;
pub use parser::{Handler, Parser};
pub use sgr::SgrStyle;
pub use sixel::encode_sixel;
pub use strip::strip_ansi;
pub use truncate::{truncate, truncate_left};
pub use vt::Terminal;
pub use width::{grapheme_width, string_height, string_width};
pub use wrap::{wordwrap, wrap};
