use crate::cellbuf::{Buffer, Cell, CellStyle, AttrMask, UnderlineStyle};
use crate::parser::{Handler, Parser};

/// VT102/xterm terminal emulator.
///
/// Maintains a cell buffer that is updated by feeding raw bytes through an
/// embedded ANSI parser. Supports cursor movement, scrolling regions, SGR
/// styling, erase operations, and alternate screen toggling.
pub struct Terminal {
    width: u16,
    height: u16,
    buffer: Buffer,
    alt_buffer: Option<Buffer>,
    cursor_x: u16,
    cursor_y: u16,
    saved_cursor: (u16, u16),
    style: CellStyle,
    auto_wrap: bool,
    origin_mode: bool,
    insert_mode: bool,
    alt_screen: bool,
    margin_top: u16,
    margin_bottom: u16,
    cursor_visible: bool,
    parser: Parser,
    tab_stops: Vec<bool>,
    /// Set when a character was written at the rightmost column and the cursor
    /// has not yet wrapped. The next printable character triggers a wrap.
    wrap_pending: bool,
}

impl Terminal {
    pub fn new(width: u16, height: u16) -> Self {
        let mut tab_stops = vec![false; width as usize];
        for i in (0..width as usize).step_by(8) {
            tab_stops[i] = true;
        }
        Self {
            width,
            height,
            buffer: Buffer::new(width as usize, height as usize),
            alt_buffer: None,
            cursor_x: 0,
            cursor_y: 0,
            saved_cursor: (0, 0),
            style: CellStyle::default(),
            auto_wrap: true,
            origin_mode: false,
            insert_mode: false,
            alt_screen: false,
            margin_top: 0,
            margin_bottom: height.saturating_sub(1),
            cursor_visible: true,
            parser: Parser::new(),
            tab_stops,
            wrap_pending: false,
        }
    }

    /// Resize the terminal. Clamps the cursor and resets scroll margins.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.buffer.resize(width as usize, height as usize);
        if let Some(ref mut alt) = self.alt_buffer {
            alt.resize(width as usize, height as usize);
        }
        self.margin_top = 0;
        self.margin_bottom = height.saturating_sub(1);
        self.cursor_x = self.cursor_x.min(width.saturating_sub(1));
        self.cursor_y = self.cursor_y.min(height.saturating_sub(1));
        self.tab_stops = vec![false; width as usize];
        for i in (0..width as usize).step_by(8) {
            self.tab_stops[i] = true;
        }
    }

    /// Feed raw bytes through the parser, updating the terminal state.
    pub fn process(&mut self, data: &[u8]) {
        // We need to split borrow: parser vs the rest of Terminal.
        // Take the parser out temporarily.
        let mut parser = std::mem::replace(&mut self.parser, Parser::new());
        {
            let mut handler = TerminalHandler { term: self };
            parser.process(&mut handler, data);
        }
        self.parser = parser;
    }

    /// Get a reference to a cell at the given position.
    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        self.buffer.cell(x as usize, y as usize)
    }

    /// Current cursor position (0-based).
    pub fn cursor_position(&self) -> (u16, u16) {
        (self.cursor_x, self.cursor_y)
    }

    /// Whether the cursor is currently visible.
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Reference to the underlying buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Render the buffer contents to a plain string (no ANSI escapes).
    /// Each row is separated by a newline, trailing blanks per row are trimmed.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity((self.width as usize + 1) * self.height as usize);
        for y in 0..self.height as usize {
            if y > 0 {
                out.push('\n');
            }
            let mut line = String::with_capacity(self.width as usize);
            for x in 0..self.width as usize {
                if let Some(cell) = self.buffer.cell(x, y) {
                    if cell.is_empty() {
                        // Wide-char continuation placeholder -- skip
                        continue;
                    }
                    line.push_str(&cell.content());
                } else {
                    line.push(' ');
                }
            }
            let trimmed = line.trim_end();
            out.push_str(trimmed);
        }
        out
    }

    // -- internal helpers --

    fn active_buffer(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    fn scroll_up(&mut self, n: u16) {
        let top = self.margin_top as usize;
        let bot = self.margin_bottom as usize;
        let count = (n as usize).min(bot - top + 1);
        self.scroll_region_up(top, bot, count);
    }

    fn scroll_down(&mut self, n: u16) {
        let top = self.margin_top as usize;
        let bot = self.margin_bottom as usize;
        let count = (n as usize).min(bot - top + 1);
        self.scroll_region_down(top, bot, count);
    }

    fn scroll_region_up(&mut self, top: usize, bot: usize, n: usize) {
        let w = self.width as usize;
        for _ in 0..n {
            // Remove the top line, shift everything up, insert blank at bottom
            for y in top..bot {
                for x in 0..w {
                    let cell = self.buffer.cell(x, y + 1).cloned().unwrap_or_else(Cell::blank);
                    if let Some(dst) = self.buffer.cell_mut(x, y) {
                        *dst = cell;
                    }
                }
            }
            // Blank the bottom line
            for x in 0..w {
                if let Some(dst) = self.buffer.cell_mut(x, bot) {
                    dst.make_blank();
                }
            }
        }
    }

    fn scroll_region_down(&mut self, top: usize, bot: usize, n: usize) {
        let w = self.width as usize;
        for _ in 0..n {
            // Shift lines down within [top..bot], insert blank at top
            for y in (top + 1..=bot).rev() {
                for x in 0..w {
                    let cell = self.buffer.cell(x, y - 1).cloned().unwrap_or_else(Cell::blank);
                    if let Some(dst) = self.buffer.cell_mut(x, y) {
                        *dst = cell;
                    }
                }
            }
            // Blank the top line
            for x in 0..w {
                if let Some(dst) = self.buffer.cell_mut(x, top) {
                    dst.make_blank();
                }
            }
        }
    }

    fn clamp_cursor(&mut self) {
        self.cursor_x = self.cursor_x.min(self.width.saturating_sub(1));
        self.cursor_y = self.cursor_y.min(self.height.saturating_sub(1));
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

struct TerminalHandler<'a> {
    term: &'a mut Terminal,
}

impl<'a> Handler for TerminalHandler<'a> {
    fn print(&mut self, ch: char) {
        let t = &mut *self.term;

        // If a wrap is pending, commit it now
        if t.wrap_pending {
            t.wrap_pending = false;
            t.cursor_x = 0;
            if t.cursor_y == t.margin_bottom {
                t.scroll_up(1);
            } else if t.cursor_y < t.height - 1 {
                t.cursor_y += 1;
            }
        }

        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        if cw == 0 {
            // Combining character: attach to previous cell
            if t.cursor_x > 0 {
                if let Some(cell) = t.buffer.cell_mut((t.cursor_x - 1) as usize, t.cursor_y as usize) {
                    cell.comb.push(ch);
                }
            }
            return;
        }

        // Handle insert mode: shift cells right
        if t.insert_mode {
            let y = t.cursor_y as usize;
            let x = t.cursor_x as usize;
            let w = t.width as usize;
            let shift = cw as usize;
            for col in (x + shift..w).rev() {
                let src = t.buffer.cell(col - shift, y).cloned().unwrap_or_else(Cell::blank);
                if let Some(dst) = t.buffer.cell_mut(col, y) {
                    *dst = src;
                }
            }
        }

        let mut cell = Cell::new(ch);
        cell.style = t.style.clone();
        t.buffer.set_cell(t.cursor_x as usize, t.cursor_y as usize, cell);

        // Advance cursor
        let new_x = t.cursor_x + cw;
        if new_x >= t.width {
            if t.auto_wrap {
                t.wrap_pending = true;
                // cursor stays at last column
                t.cursor_x = t.width.saturating_sub(1);
            }
            // else: cursor stays at right edge, next char overwrites
        } else {
            t.cursor_x = new_x;
        }
    }

    fn execute(&mut self, byte: u8) {
        let t = &mut *self.term;
        t.wrap_pending = false;
        match byte {
            // Line feed, vertical tab, form feed
            0x0A | 0x0B | 0x0C => {
                if t.cursor_y == t.margin_bottom {
                    t.scroll_up(1);
                } else if t.cursor_y < t.height - 1 {
                    t.cursor_y += 1;
                }
            }
            // Carriage return
            0x0D => {
                t.cursor_x = 0;
            }
            // Tab
            0x09 => {
                let mut next = t.cursor_x + 1;
                while (next as usize) < t.tab_stops.len() && !t.tab_stops[next as usize] {
                    next += 1;
                }
                t.cursor_x = next.min(t.width.saturating_sub(1));
            }
            // Backspace
            0x08 => {
                t.cursor_x = t.cursor_x.saturating_sub(1);
            }
            // Bell - ignore
            0x07 => {}
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &[i32], intermediates: &[u8], final_byte: u8) {
        let t = &mut *self.term;
        t.wrap_pending = false;
        let is_private = intermediates.contains(&b'?');

        match final_byte {
            // CUU - cursor up
            b'A' => {
                let n = param(params, 0, 1) as u16;
                t.cursor_y = t.cursor_y.saturating_sub(n).max(t.margin_top);
            }
            // CUD - cursor down
            b'B' => {
                let n = param(params, 0, 1) as u16;
                t.cursor_y = (t.cursor_y + n).min(t.margin_bottom);
            }
            // CUF - cursor forward
            b'C' => {
                let n = param(params, 0, 1) as u16;
                t.cursor_x = (t.cursor_x + n).min(t.width.saturating_sub(1));
            }
            // CUB - cursor backward
            b'D' => {
                let n = param(params, 0, 1) as u16;
                t.cursor_x = t.cursor_x.saturating_sub(n);
            }
            // CUP / HVP - cursor position
            b'H' | b'f' => {
                let row = param(params, 0, 1).max(1) as u16 - 1;
                let col = param(params, 1, 1).max(1) as u16 - 1;
                t.cursor_y = row.min(t.height.saturating_sub(1));
                t.cursor_x = col.min(t.width.saturating_sub(1));
            }
            // ED - erase display
            b'J' => {
                let mode = param(params, 0, 0);
                match mode {
                    0 => {
                        // Erase from cursor to end
                        erase_line_from(t, t.cursor_x, t.cursor_y);
                        for y in (t.cursor_y + 1)..t.height {
                            erase_line_from(t, 0, y);
                        }
                    }
                    1 => {
                        // Erase from start to cursor
                        for y in 0..t.cursor_y {
                            erase_line_from(t, 0, y);
                        }
                        erase_line_to(t, t.cursor_x, t.cursor_y);
                    }
                    2 | 3 => {
                        // Erase all
                        t.buffer.clear();
                    }
                    _ => {}
                }
            }
            // EL - erase line
            b'K' => {
                let mode = param(params, 0, 0);
                match mode {
                    0 => erase_line_from(t, t.cursor_x, t.cursor_y),
                    1 => erase_line_to(t, t.cursor_x, t.cursor_y),
                    2 => erase_line_from(t, 0, t.cursor_y),
                    _ => {}
                }
            }
            // IL - insert lines
            b'L' => {
                let n = param(params, 0, 1) as u16;
                if t.cursor_y >= t.margin_top && t.cursor_y <= t.margin_bottom {
                    let count = n.min(t.margin_bottom - t.cursor_y + 1);
                    t.scroll_region_down(t.cursor_y as usize, t.margin_bottom as usize, count as usize);
                }
            }
            // DL - delete lines
            b'M' => {
                let n = param(params, 0, 1) as u16;
                if t.cursor_y >= t.margin_top && t.cursor_y <= t.margin_bottom {
                    let count = n.min(t.margin_bottom - t.cursor_y + 1);
                    t.scroll_region_up(t.cursor_y as usize, t.margin_bottom as usize, count as usize);
                }
            }
            // DCH - delete characters
            b'P' => {
                let n = param(params, 0, 1) as u16;
                let y = t.cursor_y as usize;
                let x = t.cursor_x as usize;
                let w = t.width as usize;
                let count = (n as usize).min(w - x);
                // Shift cells left
                for col in x..w - count {
                    let src = t.buffer.cell(col + count, y).cloned().unwrap_or_else(Cell::blank);
                    if let Some(dst) = t.buffer.cell_mut(col, y) {
                        *dst = src;
                    }
                }
                // Blank the vacated cells at the end
                for col in (w - count)..w {
                    if let Some(dst) = t.buffer.cell_mut(col, y) {
                        dst.make_blank();
                    }
                }
            }
            // ICH - insert characters
            b'@' => {
                let n = param(params, 0, 1) as u16;
                let y = t.cursor_y as usize;
                let x = t.cursor_x as usize;
                let w = t.width as usize;
                let count = (n as usize).min(w - x);
                // Shift cells right
                for col in (x + count..w).rev() {
                    let src = t.buffer.cell(col - count, y).cloned().unwrap_or_else(Cell::blank);
                    if let Some(dst) = t.buffer.cell_mut(col, y) {
                        *dst = src;
                    }
                }
                // Blank the inserted cells
                for col in x..x + count {
                    if let Some(dst) = t.buffer.cell_mut(col, y) {
                        dst.make_blank();
                    }
                }
            }
            // SU - scroll up
            b'S' => {
                let n = param(params, 0, 1) as u16;
                t.scroll_up(n);
            }
            // SD - scroll down
            b'T' => {
                let n = param(params, 0, 1) as u16;
                t.scroll_down(n);
            }
            // SGR - select graphic rendition
            b'm' => {
                parse_sgr(params, &mut t.style);
            }
            // DECSTBM - set scroll margins
            b'r' => {
                let top = param(params, 0, 1).max(1) as u16 - 1;
                let bot = param(params, 1, t.height as i32).max(1) as u16 - 1;
                let bot = bot.min(t.height.saturating_sub(1));
                if top < bot {
                    t.margin_top = top;
                    t.margin_bottom = bot;
                }
                t.cursor_x = 0;
                t.cursor_y = if t.origin_mode { t.margin_top } else { 0 };
            }
            // SM / RM - set/reset mode
            b'h' | b'l' => {
                let set = final_byte == b'h';
                if is_private {
                    for &p in params {
                        match p {
                            7 => t.auto_wrap = set,
                            25 => t.cursor_visible = set,
                            1049 => {
                                if set {
                                    // Switch to alt screen
                                    if !t.alt_screen {
                                        t.saved_cursor = (t.cursor_x, t.cursor_y);
                                        let main = std::mem::replace(
                                            &mut t.buffer,
                                            Buffer::new(t.width as usize, t.height as usize),
                                        );
                                        t.alt_buffer = Some(main);
                                        t.alt_screen = true;
                                        t.cursor_x = 0;
                                        t.cursor_y = 0;
                                    }
                                } else {
                                    // Switch back to main screen
                                    if t.alt_screen {
                                        if let Some(main) = t.alt_buffer.take() {
                                            t.buffer = main;
                                        }
                                        t.alt_screen = false;
                                        t.cursor_x = t.saved_cursor.0;
                                        t.cursor_y = t.saved_cursor.1;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    for &p in params {
                        match p {
                            4 => t.insert_mode = set,
                            _ => {}
                        }
                    }
                }
            }
            // DECSC - save cursor (CSI s)
            b's' => {
                t.saved_cursor = (t.cursor_x, t.cursor_y);
            }
            // DECRC - restore cursor (CSI u)
            b'u' => {
                t.cursor_x = t.saved_cursor.0;
                t.cursor_y = t.saved_cursor.1;
                t.clamp_cursor();
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], final_byte: u8) {
        let t = &mut *self.term;
        t.wrap_pending = false;

        if !intermediates.is_empty() {
            return;
        }

        match final_byte {
            // DECSC - save cursor
            b'7' => {
                t.saved_cursor = (t.cursor_x, t.cursor_y);
            }
            // DECRC - restore cursor
            b'8' => {
                t.cursor_x = t.saved_cursor.0;
                t.cursor_y = t.saved_cursor.1;
                t.clamp_cursor();
            }
            // IND - index (move cursor down, scroll if at bottom margin)
            b'D' => {
                if t.cursor_y == t.margin_bottom {
                    t.scroll_up(1);
                } else if t.cursor_y < t.height - 1 {
                    t.cursor_y += 1;
                }
            }
            // RI - reverse index (move cursor up, scroll if at top margin)
            b'M' => {
                if t.cursor_y == t.margin_top {
                    t.scroll_down(1);
                } else if t.cursor_y > 0 {
                    t.cursor_y -= 1;
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get parameter at index, falling back to a default.
fn param(params: &[i32], idx: usize, default: i32) -> i32 {
    params.get(idx).copied().filter(|&v| v != 0).unwrap_or(default)
}

/// Erase from (x, y) to end of line.
fn erase_line_from(t: &mut Terminal, x: u16, y: u16) {
    for col in x as usize..t.width as usize {
        if let Some(cell) = t.buffer.cell_mut(col, y as usize) {
            cell.make_blank();
        }
    }
}

/// Erase from start of line to (x, y) inclusive.
fn erase_line_to(t: &mut Terminal, x: u16, y: u16) {
    for col in 0..=x as usize {
        if col < t.width as usize {
            if let Some(cell) = t.buffer.cell_mut(col, y as usize) {
                cell.make_blank();
            }
        }
    }
}

/// Convert ANSI 256-color index to RGB.
fn ansi_256_to_rgb(idx: u8) -> (u8, u8, u8) {
    if idx < 16 {
        ansi_basic_color(idx)
    } else if idx < 232 {
        let n = idx - 16;
        let b = (n % 6) * 51;
        let g = ((n / 6) % 6) * 51;
        let r = (n / 36) * 51;
        (r, g, b)
    } else {
        let v = 8 + (idx - 232) * 10;
        (v, v, v)
    }
}

/// Convert ANSI basic color index (0-15) to RGB.
fn ansi_basic_color(idx: u8) -> (u8, u8, u8) {
    match idx {
        0 => (0, 0, 0),
        1 => (170, 0, 0),
        2 => (0, 170, 0),
        3 => (170, 170, 0),
        4 => (0, 0, 170),
        5 => (170, 0, 170),
        6 => (0, 170, 170),
        7 => (170, 170, 170),
        8 => (85, 85, 85),
        9 => (255, 85, 85),
        10 => (85, 255, 85),
        11 => (255, 255, 85),
        12 => (85, 85, 255),
        13 => (255, 85, 255),
        14 => (85, 255, 255),
        15 => (255, 255, 255),
        _ => (0, 0, 0),
    }
}

/// Parse SGR parameters (integer slice from the parser) into a CellStyle.
fn parse_sgr(params: &[i32], style: &mut CellStyle) {
    if params.is_empty() || (params.len() == 1 && params[0] == 0) {
        *style = CellStyle::default();
        return;
    }

    let mut i = 0;
    while i < params.len() {
        let p = params[i];
        match p {
            0 => *style = CellStyle::default(),
            1 => style.attrs.set(AttrMask::BOLD),
            2 => style.attrs.set(AttrMask::FAINT),
            3 => style.attrs.set(AttrMask::ITALIC),
            4 => style.ul_style = UnderlineStyle::Single,
            5 => style.attrs.set(AttrMask::SLOW_BLINK),
            6 => style.attrs.set(AttrMask::RAPID_BLINK),
            7 => style.attrs.set(AttrMask::REVERSE),
            8 => style.attrs.set(AttrMask::CONCEAL),
            9 => style.attrs.set(AttrMask::STRIKETHROUGH),
            21 => style.ul_style = UnderlineStyle::Double,
            22 => {
                style.attrs.unset(AttrMask::BOLD);
                style.attrs.unset(AttrMask::FAINT);
            }
            23 => style.attrs.unset(AttrMask::ITALIC),
            24 => style.ul_style = UnderlineStyle::None,
            25 => {
                style.attrs.unset(AttrMask::SLOW_BLINK);
                style.attrs.unset(AttrMask::RAPID_BLINK);
            }
            27 => style.attrs.unset(AttrMask::REVERSE),
            28 => style.attrs.unset(AttrMask::CONCEAL),
            29 => style.attrs.unset(AttrMask::STRIKETHROUGH),
            // Foreground basic
            30..=37 => {
                style.fg = Some(ansi_basic_color((p - 30) as u8));
            }
            38 => {
                // Extended foreground: 38;2;r;g;b or 38;5;idx
                if let Some(color) = parse_extended_color(params, &mut i) {
                    style.fg = Some(color);
                }
            }
            39 => style.fg = None,
            // Background basic
            40..=47 => {
                style.bg = Some(ansi_basic_color((p - 40) as u8));
            }
            48 => {
                if let Some(color) = parse_extended_color(params, &mut i) {
                    style.bg = Some(color);
                }
            }
            49 => style.bg = None,
            // Underline color
            58 => {
                if let Some(color) = parse_extended_color(params, &mut i) {
                    style.ul = Some(color);
                }
            }
            59 => style.ul = None,
            // Bright foreground
            90..=97 => {
                style.fg = Some(ansi_basic_color((p - 90 + 8) as u8));
            }
            // Bright background
            100..=107 => {
                style.bg = Some(ansi_basic_color((p - 100 + 8) as u8));
            }
            _ => {}
        }
        i += 1;
    }
}

/// Parse extended color from params slice starting after the 38/48/58 marker.
/// Advances `i` past the consumed parameters.
fn parse_extended_color(params: &[i32], i: &mut usize) -> Option<(u8, u8, u8)> {
    let base = *i;
    if base + 1 >= params.len() {
        return None;
    }
    let mode = params[base + 1];
    match mode {
        2 => {
            // RGB: 38;2;r;g;b
            if base + 4 >= params.len() {
                return None;
            }
            let r = params[base + 2] as u8;
            let g = params[base + 3] as u8;
            let b = params[base + 4] as u8;
            *i = base + 4; // will be incremented by caller
            Some((r, g, b))
        }
        5 => {
            // 256-color: 38;5;idx
            if base + 2 >= params.len() {
                return None;
            }
            let idx = params[base + 2] as u8;
            *i = base + 2;
            Some(ansi_256_to_rgb(idx))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Text writing --

    #[test]
    fn test_write_text() {
        let mut term = Terminal::new(10, 3);
        term.process(b"Hello");
        assert_eq!(term.cell(0, 0).unwrap().rune, 'H');
        assert_eq!(term.cell(1, 0).unwrap().rune, 'e');
        assert_eq!(term.cell(2, 0).unwrap().rune, 'l');
        assert_eq!(term.cell(3, 0).unwrap().rune, 'l');
        assert_eq!(term.cell(4, 0).unwrap().rune, 'o');
        assert_eq!(term.cursor_position(), (5, 0));
    }

    #[test]
    fn test_write_with_newline() {
        let mut term = Terminal::new(10, 3);
        term.process(b"AB\r\nCD");
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        assert_eq!(term.cell(1, 0).unwrap().rune, 'B');
        assert_eq!(term.cell(0, 1).unwrap().rune, 'C');
        assert_eq!(term.cell(1, 1).unwrap().rune, 'D');
    }

    #[test]
    fn test_auto_wrap() {
        let mut term = Terminal::new(5, 3);
        term.process(b"ABCDEFGH");
        // "ABCDE" on row 0, "FGH" on row 1
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        assert_eq!(term.cell(4, 0).unwrap().rune, 'E');
        assert_eq!(term.cell(0, 1).unwrap().rune, 'F');
        assert_eq!(term.cell(2, 1).unwrap().rune, 'H');
    }

    #[test]
    fn test_auto_wrap_disabled() {
        let mut term = Terminal::new(5, 3);
        // Disable auto-wrap: CSI ?7l
        term.process(b"\x1b[?7l");
        term.process(b"ABCDEFGH");
        // All chars overwrite the last column
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        assert_eq!(term.cell(4, 0).unwrap().rune, 'H');
        // Cursor stays on row 0
        assert_eq!(term.cursor_position().1, 0);
    }

    // -- Cursor movement --

    #[test]
    fn test_cursor_up() {
        let mut term = Terminal::new(10, 10);
        term.process(b"\x1b[5;5H"); // move to (4,4)
        term.process(b"\x1b[2A"); // up 2
        assert_eq!(term.cursor_position(), (4, 2));
    }

    #[test]
    fn test_cursor_down() {
        let mut term = Terminal::new(10, 10);
        term.process(b"\x1b[2B"); // down 2 from (0,0)
        assert_eq!(term.cursor_position(), (0, 2));
    }

    #[test]
    fn test_cursor_forward() {
        let mut term = Terminal::new(10, 10);
        term.process(b"\x1b[3C"); // right 3
        assert_eq!(term.cursor_position(), (3, 0));
    }

    #[test]
    fn test_cursor_backward() {
        let mut term = Terminal::new(10, 10);
        term.process(b"\x1b[5;5H"); // move to (4,4)
        term.process(b"\x1b[2D"); // left 2
        assert_eq!(term.cursor_position(), (2, 4));
    }

    #[test]
    fn test_cursor_position_cup() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[10;20H");
        assert_eq!(term.cursor_position(), (19, 9));
    }

    // -- Erase operations --

    #[test]
    fn test_erase_display_below() {
        let mut term = Terminal::new(5, 3);
        term.process(b"AAAAA\r\nBBBBB\r\nCCCCC");
        term.process(b"\x1b[2;3H"); // row 2, col 3 -> (2, 1)
        term.process(b"\x1b[0J"); // erase below
        // Row 0 untouched
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        // Row 1: first 2 chars preserved, rest blank
        assert_eq!(term.cell(0, 1).unwrap().rune, 'B');
        assert_eq!(term.cell(1, 1).unwrap().rune, 'B');
        assert!(term.cell(2, 1).unwrap().is_blank());
        // Row 2: all blank
        assert!(term.cell(0, 2).unwrap().is_blank());
    }

    #[test]
    fn test_erase_display_above() {
        let mut term = Terminal::new(5, 3);
        term.process(b"AAAAA\r\nBBBBB\r\nCCCCC");
        term.process(b"\x1b[2;3H"); // -> (2, 1)
        term.process(b"\x1b[1J"); // erase above
        // Row 0: all blank
        assert!(term.cell(0, 0).unwrap().is_blank());
        // Row 1: first 3 chars blank (0,1,2 inclusive), rest preserved
        assert!(term.cell(0, 1).unwrap().is_blank());
        assert!(term.cell(2, 1).unwrap().is_blank());
        assert_eq!(term.cell(3, 1).unwrap().rune, 'B');
        // Row 2: untouched
        assert_eq!(term.cell(0, 2).unwrap().rune, 'C');
    }

    #[test]
    fn test_erase_display_all() {
        let mut term = Terminal::new(5, 3);
        term.process(b"Hello\r\nWorld");
        term.process(b"\x1b[2J");
        for y in 0..3 {
            for x in 0..5 {
                assert!(term.cell(x, y).unwrap().is_blank());
            }
        }
    }

    #[test]
    fn test_erase_line_right() {
        let mut term = Terminal::new(10, 1);
        term.process(b"ABCDEFGHIJ");
        term.process(b"\x1b[1;4H"); // -> col 3
        term.process(b"\x1b[0K");
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        assert_eq!(term.cell(2, 0).unwrap().rune, 'C');
        assert!(term.cell(3, 0).unwrap().is_blank());
        assert!(term.cell(9, 0).unwrap().is_blank());
    }

    #[test]
    fn test_erase_line_left() {
        let mut term = Terminal::new(10, 1);
        term.process(b"ABCDEFGHIJ");
        term.process(b"\x1b[1;4H"); // -> col 3
        term.process(b"\x1b[1K");
        assert!(term.cell(0, 0).unwrap().is_blank());
        assert!(term.cell(3, 0).unwrap().is_blank());
        assert_eq!(term.cell(4, 0).unwrap().rune, 'E');
    }

    #[test]
    fn test_erase_line_all() {
        let mut term = Terminal::new(10, 1);
        term.process(b"ABCDEFGHIJ");
        term.process(b"\x1b[2K");
        for x in 0..10 {
            assert!(term.cell(x, 0).unwrap().is_blank());
        }
    }

    // -- Scroll margins --

    #[test]
    fn test_scroll_margins() {
        let mut term = Terminal::new(5, 5);
        // Fill rows
        term.process(b"AAAAA\r\nBBBBB\r\nCCCCC\r\nDDDDD\r\nEEEEE");
        // Set scroll region rows 2-4 (1-indexed)
        term.process(b"\x1b[2;4r");
        // Cursor should move to (0,0)
        assert_eq!(term.cursor_position(), (0, 0));
        // Move to bottom of scroll region and force scroll
        term.process(b"\x1b[4;1H"); // row 4 = index 3
        term.process(b"\n"); // should scroll within region
        // Row 0 (outside region) should be unchanged
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        // Row 1 should now have what was row 2
        assert_eq!(term.cell(0, 1).unwrap().rune, 'C');
        // Row 2 should have what was row 3
        assert_eq!(term.cell(0, 2).unwrap().rune, 'D');
        // Row 3 should be blank (new line)
        assert!(term.cell(0, 3).unwrap().is_blank());
        // Row 4 (outside region) unchanged
        assert_eq!(term.cell(0, 4).unwrap().rune, 'E');
    }

    #[test]
    fn test_scroll_up_su() {
        let mut term = Terminal::new(5, 3);
        term.process(b"AAAAA\r\nBBBBB\r\nCCCCC");
        term.process(b"\x1b[1S"); // scroll up 1
        assert_eq!(term.cell(0, 0).unwrap().rune, 'B');
        assert_eq!(term.cell(0, 1).unwrap().rune, 'C');
        assert!(term.cell(0, 2).unwrap().is_blank());
    }

    #[test]
    fn test_scroll_down_sd() {
        let mut term = Terminal::new(5, 3);
        term.process(b"AAAAA\r\nBBBBB\r\nCCCCC");
        term.process(b"\x1b[1T"); // scroll down 1
        assert!(term.cell(0, 0).unwrap().is_blank());
        assert_eq!(term.cell(0, 1).unwrap().rune, 'A');
        assert_eq!(term.cell(0, 2).unwrap().rune, 'B');
    }

    // -- Alt screen --

    #[test]
    fn test_alt_screen_toggle() {
        let mut term = Terminal::new(10, 3);
        term.process(b"Main");
        assert_eq!(term.cell(0, 0).unwrap().rune, 'M');
        // Switch to alt screen
        term.process(b"\x1b[?1049h");
        assert!(term.alt_screen);
        // Alt screen should be blank
        assert!(term.cell(0, 0).unwrap().is_blank());
        term.process(b"Alt");
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        // Switch back
        term.process(b"\x1b[?1049l");
        assert!(!term.alt_screen);
        assert_eq!(term.cell(0, 0).unwrap().rune, 'M');
    }

    // -- SGR --

    #[test]
    fn test_sgr_bold() {
        let mut term = Terminal::new(10, 1);
        term.process(b"\x1b[1mA");
        let cell = term.cell(0, 0).unwrap();
        assert_eq!(cell.rune, 'A');
        assert!(cell.style.attrs.contains(AttrMask::BOLD));
    }

    #[test]
    fn test_sgr_reset() {
        let mut term = Terminal::new(10, 1);
        term.process(b"\x1b[1;3mA\x1b[0mB");
        let a = term.cell(0, 0).unwrap();
        assert!(a.style.attrs.contains(AttrMask::BOLD));
        assert!(a.style.attrs.contains(AttrMask::ITALIC));
        let b = term.cell(1, 0).unwrap();
        assert!(b.style.is_empty());
    }

    #[test]
    fn test_sgr_fg_rgb() {
        let mut term = Terminal::new(10, 1);
        term.process(b"\x1b[38;2;255;128;0mX");
        let cell = term.cell(0, 0).unwrap();
        assert_eq!(cell.style.fg, Some((255, 128, 0)));
    }

    #[test]
    fn test_sgr_bg_256() {
        let mut term = Terminal::new(10, 1);
        // Color 196 in 256-palette = bright red
        term.process(b"\x1b[48;5;196mX");
        let cell = term.cell(0, 0).unwrap();
        assert!(cell.style.bg.is_some());
    }

    #[test]
    fn test_sgr_basic_fg() {
        let mut term = Terminal::new(10, 1);
        term.process(b"\x1b[31mR"); // red
        let cell = term.cell(0, 0).unwrap();
        assert_eq!(cell.style.fg, Some((170, 0, 0)));
    }

    // -- ESC sequences --

    #[test]
    fn test_save_restore_cursor_esc() {
        let mut term = Terminal::new(10, 10);
        term.process(b"\x1b[5;5H"); // -> (4,4)
        term.process(b"\x1b7"); // save
        term.process(b"\x1b[1;1H"); // -> (0,0)
        term.process(b"\x1b8"); // restore
        assert_eq!(term.cursor_position(), (4, 4));
    }

    #[test]
    fn test_save_restore_cursor_csi() {
        let mut term = Terminal::new(10, 10);
        term.process(b"\x1b[5;5H"); // -> (4,4)
        term.process(b"\x1b[s"); // save
        term.process(b"\x1b[1;1H"); // -> (0,0)
        term.process(b"\x1b[u"); // restore
        assert_eq!(term.cursor_position(), (4, 4));
    }

    #[test]
    fn test_reverse_index() {
        let mut term = Terminal::new(5, 3);
        term.process(b"AAAAA\r\nBBBBB\r\nCCCCC");
        term.process(b"\x1b[1;1H"); // top-left
        term.process(b"\x1bM"); // reverse index at top should scroll down
        assert!(term.cell(0, 0).unwrap().is_blank());
        assert_eq!(term.cell(0, 1).unwrap().rune, 'A');
    }

    #[test]
    fn test_index_esc_d() {
        let mut term = Terminal::new(5, 3);
        term.process(b"AAAAA\r\nBBBBB\r\nCCCCC");
        term.process(b"\x1b[3;1H"); // bottom row
        term.process(b"\x1bD"); // index at bottom should scroll up
        assert_eq!(term.cell(0, 0).unwrap().rune, 'B');
        assert_eq!(term.cell(0, 1).unwrap().rune, 'C');
        assert!(term.cell(0, 2).unwrap().is_blank());
    }

    // -- Insert / delete --

    #[test]
    fn test_insert_chars() {
        let mut term = Terminal::new(10, 1);
        term.process(b"ABCDE");
        term.process(b"\x1b[1;2H"); // col 1 (0-indexed)
        term.process(b"\x1b[2@"); // insert 2 blanks
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        assert!(term.cell(1, 0).unwrap().is_blank());
        assert!(term.cell(2, 0).unwrap().is_blank());
        assert_eq!(term.cell(3, 0).unwrap().rune, 'B');
        assert_eq!(term.cell(4, 0).unwrap().rune, 'C');
    }

    #[test]
    fn test_delete_chars() {
        let mut term = Terminal::new(10, 1);
        term.process(b"ABCDE");
        term.process(b"\x1b[1;2H"); // col 1
        term.process(b"\x1b[2P"); // delete 2 chars
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        assert_eq!(term.cell(1, 0).unwrap().rune, 'D');
        assert_eq!(term.cell(2, 0).unwrap().rune, 'E');
        assert!(term.cell(3, 0).unwrap().is_blank());
    }

    #[test]
    fn test_insert_lines() {
        let mut term = Terminal::new(5, 3);
        term.process(b"AAAAA\r\nBBBBB\r\nCCCCC");
        term.process(b"\x1b[2;1H"); // row 1
        term.process(b"\x1b[1L"); // insert 1 line
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        assert!(term.cell(0, 1).unwrap().is_blank());
        assert_eq!(term.cell(0, 2).unwrap().rune, 'B');
    }

    #[test]
    fn test_delete_lines() {
        let mut term = Terminal::new(5, 3);
        term.process(b"AAAAA\r\nBBBBB\r\nCCCCC");
        term.process(b"\x1b[2;1H"); // row 1
        term.process(b"\x1b[1M"); // delete 1 line
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        assert_eq!(term.cell(0, 1).unwrap().rune, 'C');
        assert!(term.cell(0, 2).unwrap().is_blank());
    }

    // -- Render --

    #[test]
    fn test_render() {
        let mut term = Terminal::new(10, 3);
        term.process(b"Hello\r\nWorld");
        let rendered = term.render();
        let lines: Vec<&str> = rendered.split('\n').collect();
        assert_eq!(lines[0], "Hello");
        assert_eq!(lines[1], "World");
        assert_eq!(lines[2], "");
    }

    // -- Tab --

    #[test]
    fn test_tab() {
        let mut term = Terminal::new(20, 1);
        term.process(b"A\tB");
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        assert_eq!(term.cursor_position().0, 9); // 'B' at col 8, cursor at 9
        assert_eq!(term.cell(8, 0).unwrap().rune, 'B');
    }

    // -- Backspace --

    #[test]
    fn test_backspace() {
        let mut term = Terminal::new(10, 1);
        term.process(b"AB\x08C");
        // 'A' at 0, 'B' at 1, backspace moves to 1, 'C' overwrites 'B'
        assert_eq!(term.cell(0, 0).unwrap().rune, 'A');
        assert_eq!(term.cell(1, 0).unwrap().rune, 'C');
    }

    // -- Cursor visibility --

    #[test]
    fn test_cursor_visibility() {
        let mut term = Terminal::new(10, 10);
        assert!(term.cursor_visible());
        term.process(b"\x1b[?25l");
        assert!(!term.cursor_visible());
        term.process(b"\x1b[?25h");
        assert!(term.cursor_visible());
    }

    // -- Resize --

    #[test]
    fn test_resize() {
        let mut term = Terminal::new(10, 5);
        term.process(b"Hello");
        term.resize(20, 10);
        assert_eq!(term.cell(0, 0).unwrap().rune, 'H');
        assert_eq!(term.cell(4, 0).unwrap().rune, 'o');
    }
}
