use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

use super::buffer::Buffer;
use super::cell::Cell;
use super::geom::Rect;
use super::link::Link;
use super::style::CellStyle;
use crate::util::{ansi_256_to_rgb, ansi_basic_color};

/// Configuration options for the screen renderer.
#[derive(Debug, Clone)]
pub struct ScreenOptions {
    /// Terminal color profile (affects color output).
    pub true_color: bool,
    /// Use relative cursor movement (for inline mode).
    pub relative_cursor: bool,
    /// Whether we're in alternate screen mode.
    pub alt_screen: bool,
    /// Show the cursor.
    pub show_cursor: bool,
}

impl Default for ScreenOptions {
    fn default() -> Self {
        Self {
            true_color: true,
            relative_cursor: false,
            alt_screen: false,
            show_cursor: false,
        }
    }
}

/// Tracks the current pen/cursor state.
#[derive(Debug, Clone, Default)]
struct Cursor {
    x: u16,
    y: u16,
    style: CellStyle,
    link: Link,
}

/// Double-buffered screen renderer with diff-based output.
///
/// Maintains two buffers (current and new) and computes the minimal
/// set of ANSI escape sequences to transform the terminal from the
/// current state to the desired state.
pub struct Screen {
    /// Output buffer for ANSI sequences.
    out: Vec<u8>,
    /// What the terminal currently shows.
    curbuf: Buffer,
    /// What we want the terminal to show.
    newbuf: Buffer,
    /// Current cursor/pen state.
    cur: Cursor,
    /// Width of the terminal.
    width: u16,
    /// Height of the terminal.
    height: u16,
    /// Hash values for current buffer lines.
    oldhash: Vec<u64>,
    /// Hash values for new buffer lines.
    newhash: Vec<u64>,
    /// Mapping: for each new line, which old line index it came from (-1 = new/changed).
    oldnum: Vec<i32>,
    /// Options.
    opts: ScreenOptions,
    /// Force full redraw on next render.
    force_clear: bool,
    /// Whether synchronized output is enabled.
    syncd_updates: bool,
    /// Cursor is at phantom position (past right edge after writing last column).
    at_phantom: bool,
    /// Skip scroll optimization (needed when using region-based rendering,
    /// because CSI S/T scroll the entire terminal width).
    no_scroll_optimize: bool,
}

/// Result from parsing an ANSI escape — may request a cursor position change.
#[derive(Default)]
struct EscapeResult {
    new_x: Option<usize>,
}

impl Screen {
    /// Create a new screen renderer.
    pub fn new(width: u16, height: u16) -> Self {
        let w = width as usize;
        let h = height as usize;
        Self {
            out: Vec::with_capacity(4096),
            curbuf: Buffer::new(w, h),
            newbuf: Buffer::new(w, h),
            cur: Cursor::default(),
            width,
            height,
            oldhash: vec![0; h],
            newhash: vec![0; h],
            oldnum: vec![-1; h],
            opts: ScreenOptions::default(),
            force_clear: true, // First render should be full
            syncd_updates: false,
            at_phantom: false,
            no_scroll_optimize: false,
        }
    }

    /// Set synchronized output mode (mode 2026).
    pub fn set_syncd_updates(&mut self, enabled: bool) {
        self.syncd_updates = enabled;
    }

    /// Disable scroll optimization (CSI S/T scroll the entire terminal width,
    /// which corrupts region-based layouts where only part of a row should scroll).
    pub fn set_scroll_optimize(&mut self, enabled: bool) {
        self.no_scroll_optimize = !enabled;
    }

    pub fn set_options(&mut self, opts: ScreenOptions) {
        self.opts = opts;
    }

    /// Resize the screen.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        let w = width as usize;
        let h = height as usize;
        self.curbuf.resize(w, h);
        self.newbuf.resize(w, h);
        self.oldhash.resize(h, 0);
        self.newhash.resize(h, 0);
        self.oldnum.resize(h, -1);
        self.force_clear = true;
    }

    /// Get a mutable reference to the new buffer for writing content.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.newbuf
    }

    /// Get the new buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.newbuf
    }

    /// Mark all lines as needing redraw.
    pub fn clear(&mut self) {
        self.force_clear = true;
    }

    /// Set content from a styled string. Clears the buffer first.
    /// Parses the string into the new buffer, handling ANSI escape sequences.
    pub fn set_content(&mut self, content: &str) {
        self.newbuf.clear();
        let w = self.width as usize;
        let h = self.height as usize;
        parse_content_into_buffer(&mut self.newbuf, content, 0, 0, w, h);
    }

    /// Draw a styled string into a rectangular region of the buffer.
    /// Does NOT clear the buffer — multiple calls compose naturally.
    pub fn draw_region(&mut self, content: &str, rect: Rect) {
        let ox = rect.x as usize;
        let oy = rect.y as usize;
        let mx = (rect.right() as usize).min(self.width as usize);
        let my = (rect.bottom() as usize).min(self.height as usize);
        parse_content_into_buffer(&mut self.newbuf, content, ox, oy, mx, my);
    }

    /// Clear a rectangular region of the buffer to blank cells.
    pub fn clear_region(&mut self, rect: Rect) {
        self.newbuf.clear_rect(rect);
    }

    /// Render the diff between current and new buffer, writing ANSI to `w`.
    pub fn render(&mut self, w: &mut dyn Write) -> io::Result<()> {
        self.out.clear();

        if self.syncd_updates {
            self.out.extend_from_slice(b"\x1b[?2026h");
        }

        if self.force_clear {
            self.render_full();
            self.force_clear = false;
        } else {
            self.compute_hashes();
            self.match_lines();
            if !self.no_scroll_optimize {
                self.scroll_optimize();
            }
            self.render_changed_lines();
        }

        if self.syncd_updates {
            self.out.extend_from_slice(b"\x1b[?2026l");
        }

        w.write_all(&self.out)?;
        w.flush()?;

        // Swap: new becomes current
        std::mem::swap(&mut self.curbuf, &mut self.newbuf);
        self.newbuf.clear();
        Ok(())
    }

    /// Full screen render (used on first frame or after clear).
    fn render_full(&mut self) {
        // Hide cursor
        self.out.extend_from_slice(b"\x1b[?25l");
        // Home cursor
        self.out.extend_from_slice(b"\x1b[H");
        // Clear screen
        self.out.extend_from_slice(b"\x1b[2J");

        self.cur = Cursor::default();

        let h = self.height as usize;
        let w = self.width as usize;

        for y in 0..h {
            if y > 0 {
                // Move to start of next line
                self.out.extend_from_slice(b"\r\n");
                self.cur.x = 0;
                self.cur.y += 1;
            }

            for x in 0..w {
                let cell = self.newbuf.cell(x, y).cloned().unwrap_or_default();
                if cell.is_empty() {
                    continue; // Skip wide-cell placeholders
                }
                self.emit_cell(&cell);
            }
        }

        // Reset style at end
        self.out.extend_from_slice(b"\x1b[0m");
        self.cur.style = CellStyle::default();
        self.cur.link = Link::default();

        // Show cursor if needed
        if self.opts.show_cursor {
            self.out.extend_from_slice(b"\x1b[?25h");
        }
    }

    /// Compute hashes for all lines in both buffers.
    fn compute_hashes(&mut self) {
        let h = self.height as usize;
        for y in 0..h {
            self.oldhash[y] = self.curbuf.line_hash(y);
            self.newhash[y] = self.newbuf.line_hash(y);
        }
    }

    /// Match lines between old and new buffers using hash-based algorithm.
    fn match_lines(&mut self) {
        let h = self.height as usize;

        // Reset oldnum
        for i in 0..h {
            self.oldnum[i] = -1;
        }

        // Build hash occurrence counts
        let mut old_counts: HashMap<u64, Vec<usize>> = HashMap::new();
        let mut new_counts: HashMap<u64, Vec<usize>> = HashMap::new();

        for y in 0..h {
            old_counts.entry(self.oldhash[y]).or_default().push(y);
            new_counts.entry(self.newhash[y]).or_default().push(y);
        }

        // Match unique hash pairs
        for (hash, new_indices) in &new_counts {
            if new_indices.len() == 1
                && let Some(old_indices) = old_counts.get(hash)
                && old_indices.len() == 1
            {
                self.oldnum[new_indices[0]] = old_indices[0] as i32;
            }
        }

        // Grow matched hunks forward
        for y in 1..h {
            if self.oldnum[y] == -1 && self.oldnum[y - 1] >= 0 {
                let prev_old = self.oldnum[y - 1] as usize + 1;
                if prev_old < h && self.newhash[y] == self.oldhash[prev_old] {
                    self.oldnum[y] = prev_old as i32;
                }
            }
        }

        // Grow matched hunks backward
        for y in (0..h.saturating_sub(1)).rev() {
            if self.oldnum[y] == -1 && y + 1 < h && self.oldnum[y + 1] > 0 {
                let next_old = self.oldnum[y + 1] as usize - 1;
                if self.newhash[y] == self.oldhash[next_old] {
                    self.oldnum[y] = next_old as i32;
                }
            }
        }
    }

    /// Apply scroll optimizations using matched line positions.
    fn scroll_optimize(&mut self) {
        let h = self.height as usize;

        // Find lines that can be scrolled (shifted up or down)
        // Pass 1: top to bottom - find upward scrolls (positive shift)
        let mut y = 0;
        while y < h {
            if self.oldnum[y] >= 0 {
                let shift = self.oldnum[y] - y as i32;
                if shift > 0 {
                    // Lines shifted down — need to scroll up (delete at top, insert at bottom)
                    let mut count = 1;
                    while y + count < h
                        && self.oldnum[y + count] >= 0
                        && (self.oldnum[y + count] - (y + count) as i32) == shift
                    {
                        count += 1;
                    }

                    if count >= 2 {
                        // Worth scrolling
                        self.emit_scroll_up(shift as usize);
                        // Update current buffer state
                        self.curbuf.delete_line(y, shift as usize);
                        y += count;
                        continue;
                    }
                }
            }
            y += 1;
        }

        // Pass 2: bottom to top - find downward scrolls (negative shift)
        let mut y = h;
        while y > 0 {
            y -= 1;
            if self.oldnum[y] >= 0 {
                let shift = self.oldnum[y] - y as i32;
                if shift < 0 {
                    let mut start = y;
                    while start > 0
                        && self.oldnum[start - 1] >= 0
                        && (self.oldnum[start - 1] - (start - 1) as i32) == shift
                    {
                        start -= 1;
                    }
                    let count = y - start + 1;

                    if count >= 2 {
                        let n = (-shift) as usize;
                        self.emit_scroll_down(n);
                        self.curbuf.insert_line(start, n);
                        y = start;
                        continue;
                    }
                }
            }
        }
    }

    /// Render lines that differ between old and new buffers.
    fn render_changed_lines(&mut self) {
        // Hide cursor during updates
        self.out.extend_from_slice(b"\x1b[?25l");

        let h = self.height as usize;
        let w = self.width as usize;

        for y in 0..h {
            // Check if line is unchanged (same hash AND same oldnum mapping)
            if self.oldnum[y] == y as i32 && self.oldhash[y] == self.newhash[y] {
                // Verify cells actually match (hash collision check)
                let mut same = true;
                for x in 0..w {
                    let old_cell = self.curbuf.cell(x, y);
                    let new_cell = self.newbuf.cell(x, y);
                    if old_cell != new_cell {
                        same = false;
                        break;
                    }
                }
                if same {
                    continue;
                }
            }

            // Line changed — transform it
            self.transform_line(y);
        }

        // Reset style
        self.out.extend_from_slice(b"\x1b[0m");
        self.cur.style = CellStyle::default();
        self.cur.link = Link::default();

        // Show cursor if needed
        if self.opts.show_cursor {
            self.out.extend_from_slice(b"\x1b[?25h");
        }
    }

    /// Compute minimal diff for a single line.
    fn transform_line(&mut self, y: usize) {
        let w = self.width as usize;

        // Find first differing cell
        let mut first_diff = w;
        for x in 0..w {
            if self.curbuf.cell(x, y) != self.newbuf.cell(x, y) {
                first_diff = x;
                break;
            }
        }

        if first_diff == w {
            return; // No differences
        }

        // Find last differing cell
        let mut last_diff = first_diff;
        for x in (first_diff..w).rev() {
            if self.curbuf.cell(x, y) != self.newbuf.cell(x, y) {
                last_diff = x;
                break;
            }
        }

        // Move cursor to first diff position
        self.move_cursor(first_diff as u16, y as u16);

        // Check if we can optimize with erase-to-end-of-line
        let mut can_erase_right = true;
        for x in (last_diff + 1)..w {
            let cell = self.newbuf.cell(x, y).cloned().unwrap_or_default();
            if !cell.is_blank() {
                can_erase_right = false;
                break;
            }
        }

        // Emit changed cells
        for x in first_diff..=last_diff {
            let cell = self.newbuf.cell(x, y).cloned().unwrap_or_default();
            if cell.is_empty() {
                continue; // Skip wide-cell placeholders
            }
            self.emit_cell(&cell);
        }

        // If trailing cells are all blank, erase to end of line
        if can_erase_right && last_diff < w - 1 {
            // Reset style for erase
            let diff = CellStyle::default().diff_sequence(&self.cur.style);
            if !diff.is_empty() {
                self.out.extend_from_slice(diff.as_bytes());
                self.cur.style = CellStyle::default();
            }
            self.out.extend_from_slice(b"\x1b[K"); // Erase to end of line
        }
    }

    /// Emit a single cell's content with style changes.
    fn emit_cell(&mut self, cell: &Cell) {
        // Apply style diff
        let style_diff = cell.style.diff_sequence(&self.cur.style);
        if !style_diff.is_empty() {
            self.out.extend_from_slice(style_diff.as_bytes());
            self.cur.style = cell.style.clone();
        }

        // Apply link diff
        if cell.link != self.cur.link {
            if !self.cur.link.is_empty() {
                self.out
                    .extend_from_slice(Link::close_sequence().as_bytes());
            }
            if !cell.link.is_empty() {
                self.out
                    .extend_from_slice(cell.link.open_sequence().as_bytes());
            }
            self.cur.link = cell.link.clone();
        }

        // Write cell content
        let content = cell.content();
        self.out.extend_from_slice(content.as_bytes());
        self.cur.x += cell.width as u16;

        // Check phantom state
        if self.cur.x >= self.width {
            self.at_phantom = true;
            self.cur.x = self.width - 1;
        }
    }

    /// Move cursor to target position using the shortest sequence.
    fn move_cursor(&mut self, tx: u16, ty: u16) {
        if self.cur.x == tx && self.cur.y == ty && !self.at_phantom {
            return;
        }

        self.at_phantom = false;

        // Method 1: Absolute CUP (always works)
        let abs_seq = format!("\x1b[{};{}H", ty + 1, tx + 1);
        let mut best = abs_seq.clone();

        // Method 2: Relative movements
        let mut rel = String::new();
        let dx = tx as i32 - self.cur.x as i32;
        let dy = ty as i32 - self.cur.y as i32;

        if dy != 0 {
            if dy > 0 {
                if dy == 1 {
                    rel.push_str("\x1b[B");
                } else {
                    let _ = write!(rel, "\x1b[{}B", dy);
                }
            } else if dy == -1 {
                rel.push_str("\x1b[A");
            } else {
                let _ = write!(rel, "\x1b[{}A", -dy);
            }
        }

        if dx != 0 {
            if dx > 0 {
                if dx == 1 {
                    rel.push_str("\x1b[C");
                } else {
                    let _ = write!(rel, "\x1b[{}C", dx);
                }
            } else if dx == -1 {
                rel.push_str("\x1b[D");
            } else {
                let _ = write!(rel, "\x1b[{}D", -dx);
            }
        }

        if !rel.is_empty() && rel.len() < best.len() {
            best = rel;
        }

        // Method 3: CR + relative vertical + horizontal
        if tx > 0 {
            let mut cr_rel = String::from("\r");
            if dy != 0 {
                if dy > 0 {
                    if dy == 1 {
                        cr_rel.push_str("\x1b[B");
                    } else {
                        let _ = write!(cr_rel, "\x1b[{}B", dy);
                    }
                } else if dy == -1 {
                    cr_rel.push_str("\x1b[A");
                } else {
                    let _ = write!(cr_rel, "\x1b[{}A", -dy);
                }
            }
            if tx == 1 {
                cr_rel.push_str("\x1b[C");
            } else {
                let _ = write!(cr_rel, "\x1b[{}C", tx);
            }
            if cr_rel.len() < best.len() {
                best = cr_rel;
            }
        } else if tx == 0 {
            // Just CR + vertical
            let mut cr = String::from("\r");
            if dy != 0 {
                if dy > 0 {
                    if dy == 1 {
                        cr.push_str("\x1b[B");
                    } else {
                        let _ = write!(cr, "\x1b[{}B", dy);
                    }
                } else if dy == -1 {
                    cr.push_str("\x1b[A");
                } else {
                    let _ = write!(cr, "\x1b[{}A", -dy);
                }
            }
            if cr.len() < best.len() {
                best = cr;
            }
        }

        self.out.extend_from_slice(best.as_bytes());
        self.cur.x = tx;
        self.cur.y = ty;
    }

    /// Emit scroll up command.
    fn emit_scroll_up(&mut self, n: usize) {
        // Use CSI S (scroll up)
        if n == 1 {
            self.out.extend_from_slice(b"\x1b[S");
        } else {
            let _ = write!(self.out, "\x1b[{}S", n);
        }
    }

    /// Emit scroll down command.
    fn emit_scroll_down(&mut self, n: usize) {
        // Use CSI T (scroll down)
        if n == 1 {
            self.out.extend_from_slice(b"\x1b[T");
        } else {
            let _ = write!(self.out, "\x1b[{}T", n);
        }
    }
}

/// Parse an ANSI-styled string and write cells into a buffer within the given bounds.
///
/// Content is placed starting at `(origin_x, origin_y)`. Lines wrap at `max_x` and
/// reset to `origin_x`. Parsing stops when `y >= max_y`.
fn parse_content_into_buffer(
    buffer: &mut Buffer,
    content: &str,
    origin_x: usize,
    origin_y: usize,
    max_x: usize,
    max_y: usize,
) {
    let mut x = origin_x;
    let mut y = origin_y;

    let bytes = content.as_bytes();
    let mut i = 0;
    let mut current_style = CellStyle::default();
    let mut current_link = Link::default();

    while i < bytes.len() && y < max_y {
        let b = bytes[i];

        if b == b'\x1b' {
            let (new_i, esc_result) =
                parse_escape_impl(bytes, i, &mut current_style, &mut current_link);
            i = new_i;
            if let Some(new_x) = esc_result.new_x {
                // CHA is region-relative
                x = (origin_x + new_x).min(max_x);
            }
            continue;
        }

        if b == b'\n' {
            y += 1;
            x = origin_x;
            i += 1;
            continue;
        }

        if b == b'\r' {
            x = origin_x;
            i += 1;
            continue;
        }

        if b == b'\t' {
            let rel = x - origin_x;
            let next_tab = ((rel + 8) & !7) + origin_x;
            while x < next_tab && x < max_x {
                buffer.set_cell(x, y, Cell::blank().with_style(current_style.clone()));
                x += 1;
            }
            i += 1;
            continue;
        }

        // Decode UTF-8 character
        let ch;
        let char_len;
        if b < 0x80 {
            ch = b as char;
            char_len = 1;
        } else if b < 0xE0 {
            if i + 1 < bytes.len() {
                ch = core::str::from_utf8(&bytes[i..i + 2])
                    .ok()
                    .and_then(|s| s.chars().next())
                    .unwrap_or('?');
                char_len = 2;
            } else {
                i += 1;
                continue;
            }
        } else if b < 0xF0 {
            if i + 2 < bytes.len() {
                ch = core::str::from_utf8(&bytes[i..i + 3])
                    .ok()
                    .and_then(|s| s.chars().next())
                    .unwrap_or('?');
                char_len = 3;
            } else {
                i += 1;
                continue;
            }
        } else if i + 3 < bytes.len() {
            ch = core::str::from_utf8(&bytes[i..i + 4])
                .ok()
                .and_then(|s| s.chars().next())
                .unwrap_or('?');
            char_len = 4;
        } else {
            i += 1;
            continue;
        }
        i += char_len;

        let cell = Cell::new(ch)
            .with_style(current_style.clone())
            .with_link(current_link.clone());

        let cell_w = cell.width as usize;
        if x + cell_w <= max_x {
            buffer.set_cell(x, y, cell);
            x += cell_w;
        } else {
            // Doesn't fit — move to next line
            y += 1;
            x = origin_x;
            if y < max_y {
                buffer.set_cell(x, y, cell);
                x += cell_w;
            }
        }
    }
}

/// Parse an ANSI escape sequence from bytes, updating style state.
fn parse_escape_impl(
    bytes: &[u8],
    start: usize,
    style: &mut CellStyle,
    link: &mut Link,
) -> (usize, EscapeResult) {
    let len = bytes.len();
    let mut i = start + 1; // Skip ESC
    let mut result = EscapeResult::default();

    if i >= len {
        return (i, result);
    }

    match bytes[i] {
        b'[' => {
            // CSI sequence
            i += 1;
            let params_start = i;

            // Collect parameter bytes
            while i < len
                && (bytes[i].is_ascii_digit()
                    || bytes[i] == b';'
                    || bytes[i] == b':'
                    || bytes[i] == b'?')
            {
                i += 1;
            }

            // Final byte
            if i < len {
                let final_byte = bytes[i];
                i += 1;

                if final_byte == b'm' {
                    // SGR sequence
                    parse_sgr_impl(&bytes[params_start..i - 1], style);
                } else if final_byte == b'G' {
                    // CHA — Cursor Horizontal Absolute: move to column n (1-based)
                    let param_str = std::str::from_utf8(&bytes[params_start..i - 1]).unwrap_or("1");
                    let col = param_str.parse::<usize>().unwrap_or(1);
                    result.new_x = Some(col.saturating_sub(1)); // 1-based → 0-based
                }
            }
        }
        b']' => {
            // OSC sequence — find ST (ESC \ or BEL)
            i += 1;
            let osc_start = i;
            while i < len {
                if bytes[i] == 0x07 {
                    // BEL terminator
                    parse_osc_impl(&bytes[osc_start..i], link);
                    i += 1;
                    break;
                }
                if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'\\' {
                    // ST terminator
                    parse_osc_impl(&bytes[osc_start..i], link);
                    i += 2;
                    break;
                }
                i += 1;
            }
        }
        _ => {
            // Other escape — skip
            i += 1;
        }
    }

    (i, result)
}

/// Parse SGR parameters and update cell style.
fn parse_sgr_impl(params: &[u8], style: &mut CellStyle) {
    let param_str = std::str::from_utf8(params).unwrap_or("");
    if param_str.is_empty() {
        *style = CellStyle::default();
        return;
    }

    let mut parts = param_str.split(';');
    while let Some(p) = parts.next() {
        if p.contains(':') {
            let sub: Vec<&str> = p.split(':').collect();
            if sub.first() == Some(&"4") {
                match sub.get(1).and_then(|s| s.parse::<u8>().ok()) {
                    Some(0) => style.ul_style = super::style::UnderlineStyle::None,
                    Some(1) => style.ul_style = super::style::UnderlineStyle::Single,
                    Some(2) => style.ul_style = super::style::UnderlineStyle::Double,
                    Some(3) => style.ul_style = super::style::UnderlineStyle::Curly,
                    Some(4) => style.ul_style = super::style::UnderlineStyle::Dotted,
                    Some(5) => style.ul_style = super::style::UnderlineStyle::Dashed,
                    _ => {}
                }
            }
            continue;
        }

        match p.parse::<u32>().unwrap_or(0) {
            0 => *style = CellStyle::default(),
            1 => style.attrs.set(super::style::AttrMask::BOLD),
            2 => style.attrs.set(super::style::AttrMask::FAINT),
            3 => style.attrs.set(super::style::AttrMask::ITALIC),
            4 => style.ul_style = super::style::UnderlineStyle::Single,
            5 => style.attrs.set(super::style::AttrMask::SLOW_BLINK),
            6 => style.attrs.set(super::style::AttrMask::RAPID_BLINK),
            7 => style.attrs.set(super::style::AttrMask::REVERSE),
            8 => style.attrs.set(super::style::AttrMask::CONCEAL),
            9 => style.attrs.set(super::style::AttrMask::STRIKETHROUGH),
            21 => style.ul_style = super::style::UnderlineStyle::Double,
            22 => {
                style.attrs.unset(super::style::AttrMask::BOLD);
                style.attrs.unset(super::style::AttrMask::FAINT);
            }
            23 => style.attrs.unset(super::style::AttrMask::ITALIC),
            24 => style.ul_style = super::style::UnderlineStyle::None,
            25 => {
                style.attrs.unset(super::style::AttrMask::SLOW_BLINK);
                style.attrs.unset(super::style::AttrMask::RAPID_BLINK);
            }
            27 => style.attrs.unset(super::style::AttrMask::REVERSE),
            28 => style.attrs.unset(super::style::AttrMask::CONCEAL),
            29 => style.attrs.unset(super::style::AttrMask::STRIKETHROUGH),
            30..=37 => {
                let idx = p.parse::<u32>().unwrap() - 30;
                style.fg = Some(ansi_basic_color(idx as u8));
            }
            38 => {
                if let Some(color) = parse_extended_color(&mut parts) {
                    style.fg = Some(color);
                }
            }
            39 => style.fg = None,
            40..=47 => {
                let idx = p.parse::<u32>().unwrap() - 40;
                style.bg = Some(ansi_basic_color(idx as u8));
            }
            48 => {
                if let Some(color) = parse_extended_color(&mut parts) {
                    style.bg = Some(color);
                }
            }
            49 => style.bg = None,
            58 => {
                if let Some(color) = parse_extended_color(&mut parts) {
                    style.ul = Some(color);
                }
            }
            59 => style.ul = None,
            90..=97 => {
                let idx = p.parse::<u32>().unwrap() - 90 + 8;
                style.fg = Some(ansi_basic_color(idx as u8));
            }
            100..=107 => {
                let idx = p.parse::<u32>().unwrap() - 100 + 8;
                style.bg = Some(ansi_basic_color(idx as u8));
            }
            _ => {}
        }
    }
}

/// Parse OSC sequence for hyperlinks.
fn parse_osc_impl(data: &[u8], link: &mut Link) {
    let s = std::str::from_utf8(data).unwrap_or("");
    if let Some(rest) = s.strip_prefix("8;")
        && let Some(semi) = rest.find(';')
    {
        let params = &rest[..semi];
        let url = &rest[semi + 1..];
        if url.is_empty() {
            link.reset();
        } else {
            *link = Link::with_params(url, params);
        }
    }
}

/// Parse extended color (38;2;r;g;b or 38;5;idx).
fn parse_extended_color<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Option<(u8, u8, u8)> {
    let mode = parts.next()?.parse::<u8>().ok()?;
    match mode {
        2 => {
            let r = parts.next()?.parse::<u8>().ok()?;
            let g = parts.next()?.parse::<u8>().ok()?;
            let b = parts.next()?.parse::<u8>().ok()?;
            Some((r, g, b))
        }
        5 => {
            let idx = parts.next()?.parse::<u8>().ok()?;
            Some(ansi_256_to_rgb(idx))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_creation() {
        let screen = Screen::new(80, 24);
        assert_eq!(screen.width, 80);
        assert_eq!(screen.height, 24);
    }

    #[test]
    fn test_set_content_simple() {
        let mut screen = Screen::new(10, 3);
        screen.set_content("Hello\nWorld");
        assert_eq!(screen.newbuf.cell(0, 0).unwrap().rune, 'H');
        assert_eq!(screen.newbuf.cell(4, 0).unwrap().rune, 'o');
        assert_eq!(screen.newbuf.cell(0, 1).unwrap().rune, 'W');
    }

    #[test]
    fn test_set_content_with_ansi() {
        let mut screen = Screen::new(20, 1);
        screen.set_content("\x1b[1mBold\x1b[0m");
        let cell = screen.newbuf.cell(0, 0).unwrap();
        assert_eq!(cell.rune, 'B');
        assert!(
            cell.style
                .attrs
                .contains(super::super::style::AttrMask::BOLD)
        );
    }

    #[test]
    fn test_render_full() {
        let mut screen = Screen::new(5, 2);
        screen.set_content("Hello\nWorld");
        let mut out = Vec::new();
        screen.render(&mut out).unwrap();
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("Hello"));
        assert!(output.contains("World"));
    }

    #[test]
    fn test_render_diff() {
        let mut screen = Screen::new(5, 1);
        let mut out = Vec::new();

        // First render
        screen.set_content("Hello");
        screen.render(&mut out).unwrap();

        // Second render with small change
        out.clear();
        screen.set_content("Hallo");
        screen.render(&mut out).unwrap();

        // Should be a small diff, not full redraw
        let output = String::from_utf8_lossy(&out);
        // The diff should move cursor and output "allo" (or just the changed chars)
        assert!(output.len() < 50); // Much smaller than full redraw
    }

    #[test]
    fn test_move_cursor_optimization() {
        let mut screen = Screen::new(80, 24);
        // Test that relative movement is chosen for short distances
        screen.cur = Cursor {
            x: 5,
            y: 5,
            style: CellStyle::default(),
            link: Link::default(),
        };
        screen.out.clear();
        screen.move_cursor(6, 5); // 1 column right
        let output = String::from_utf8_lossy(&screen.out);
        assert_eq!(output, "\x1b[C"); // CUF(1)
    }

    #[test]
    fn test_syncd_updates() {
        let mut screen = Screen::new(5, 1);
        screen.set_syncd_updates(true);
        screen.set_content("Hello");
        let mut out = Vec::new();
        screen.render(&mut out).unwrap();
        let output = String::from_utf8_lossy(&out);
        assert!(output.starts_with("\x1b[?2026h"));
        assert!(output.ends_with("\x1b[?2026l"));
    }

    #[test]
    fn test_resize() {
        let mut screen = Screen::new(10, 5);
        screen.set_content("Hello");
        let mut out = Vec::new();
        screen.render(&mut out).unwrap();

        screen.resize(20, 10);
        out.clear();
        screen.set_content("Hello World");
        screen.render(&mut out).unwrap();
        assert!(!out.is_empty());
    }

    #[test]
    fn test_draw_region_basic() {
        use super::super::geom::Rect;
        let mut screen = Screen::new(20, 5);
        screen.draw_region("Hello", Rect::new(5, 2, 10, 2));
        // Content at (5,2)
        assert_eq!(screen.newbuf.cell(5, 2).unwrap().rune, 'H');
        assert_eq!(screen.newbuf.cell(9, 2).unwrap().rune, 'o');
        // Outside region is blank
        assert!(screen.newbuf.cell(0, 0).unwrap().is_blank());
    }

    #[test]
    fn test_draw_region_newline_resets_to_origin() {
        use super::super::geom::Rect;
        let mut screen = Screen::new(20, 5);
        screen.draw_region("AB\nCD", Rect::new(3, 1, 10, 3));
        assert_eq!(screen.newbuf.cell(3, 1).unwrap().rune, 'A');
        assert_eq!(screen.newbuf.cell(4, 1).unwrap().rune, 'B');
        // After newline, x resets to origin_x (3), not 0
        assert_eq!(screen.newbuf.cell(3, 2).unwrap().rune, 'C');
        assert_eq!(screen.newbuf.cell(4, 2).unwrap().rune, 'D');
    }

    #[test]
    fn test_draw_region_clips_at_bounds() {
        use super::super::geom::Rect;
        let mut screen = Screen::new(20, 5);
        screen.draw_region("ABCDEFGHIJ", Rect::new(0, 0, 5, 1));
        // Only first 5 chars fit (wraps to next line but height is 1)
        assert_eq!(screen.newbuf.cell(0, 0).unwrap().rune, 'A');
        assert_eq!(screen.newbuf.cell(4, 0).unwrap().rune, 'E');
        // Row 1 should be blank (region height is 1)
        assert!(screen.newbuf.cell(0, 1).unwrap().is_blank());
    }

    #[test]
    fn test_draw_region_with_ansi() {
        use super::super::geom::Rect;
        let mut screen = Screen::new(20, 5);
        screen.draw_region("\x1b[1mBold\x1b[0m", Rect::new(2, 1, 10, 2));
        let cell = screen.newbuf.cell(2, 1).unwrap();
        assert_eq!(cell.rune, 'B');
        assert!(
            cell.style
                .attrs
                .contains(super::super::style::AttrMask::BOLD)
        );
    }

    #[test]
    fn test_draw_region_multiple_non_overlapping() {
        use super::super::geom::Rect;
        let mut screen = Screen::new(20, 5);
        screen.draw_region("Left", Rect::new(0, 0, 10, 5));
        screen.draw_region("Right", Rect::new(10, 0, 10, 5));
        assert_eq!(screen.newbuf.cell(0, 0).unwrap().rune, 'L');
        assert_eq!(screen.newbuf.cell(10, 0).unwrap().rune, 'R');
    }

    #[test]
    fn test_clear_region() {
        use super::super::geom::Rect;
        let mut screen = Screen::new(10, 3);
        screen.set_content("AAAAAAAAAA\nBBBBBBBBBB\nCCCCCCCCCC");
        screen.clear_region(Rect::new(2, 1, 5, 1));
        assert_eq!(screen.newbuf.cell(0, 1).unwrap().rune, 'B');
        assert!(screen.newbuf.cell(2, 1).unwrap().is_blank());
        assert!(screen.newbuf.cell(6, 1).unwrap().is_blank());
        assert_eq!(screen.newbuf.cell(7, 1).unwrap().rune, 'B');
    }
}
