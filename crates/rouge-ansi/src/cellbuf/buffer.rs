use super::cell::Cell;
use super::geom::Rect;

/// A line is a vector of cells.
pub type Line = Vec<Cell>;

/// A 2D grid of cells representing terminal content.
#[derive(Debug, Clone)]
pub struct Buffer {
    lines: Vec<Line>,
    width: usize,
    height: usize,
}

impl Buffer {
    /// Create a new buffer filled with blank cells.
    pub fn new(width: usize, height: usize) -> Self {
        let lines = (0..height)
            .map(|_| (0..width).map(|_| Cell::blank()).collect())
            .collect();
        Self {
            lines,
            width,
            height,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Get a reference to a line.
    pub fn line(&self, y: usize) -> Option<&Line> {
        self.lines.get(y)
    }

    /// Get a mutable reference to a line.
    pub fn line_mut(&mut self, y: usize) -> Option<&mut Line> {
        self.lines.get_mut(y)
    }

    /// Get a reference to a cell.
    pub fn cell(&self, x: usize, y: usize) -> Option<&Cell> {
        self.lines.get(y).and_then(|line| line.get(x))
    }

    /// Get a mutable reference to a cell.
    pub fn cell_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> {
        self.lines.get_mut(y).and_then(|line| line.get_mut(x))
    }

    /// Set a cell at position (x, y), handling wide-cell layout.
    ///
    /// If the cell has width > 1, subsequent positions are filled with
    /// empty placeholder cells. If the cell overwrites part of an existing
    /// wide cell, the old wide cell's positions are blanked.
    pub fn set_cell(&mut self, x: usize, y: usize, cell: Cell) {
        if y >= self.height || x >= self.width {
            return;
        }
        let cell_width = cell.width as usize;
        let line = &mut self.lines[y];

        // Clear any wide cell that originates at this position
        let existing_width = line[x].width as usize;
        if existing_width > 1 {
            for i in 0..existing_width {
                if x + i < self.width {
                    line[x + i].make_blank();
                }
            }
        }

        // If we're placing in a position that's a wide-cell placeholder,
        // blank the originator too
        if x > 0 && line[x].is_empty() {
            for i in (0..x).rev() {
                if !line[i].is_empty() {
                    let orig_width = line[i].width as usize;
                    if i + orig_width > x {
                        for j in i..i + orig_width {
                            if j < self.width {
                                line[j].make_blank();
                            }
                        }
                    }
                    break;
                }
            }
        }

        // Place the cell
        line[x] = cell;

        // Place empty placeholders for wide cells
        for i in 1..cell_width {
            if x + i < self.width {
                line[x + i] = Cell::empty();
            }
        }
    }

    /// Resize the buffer, preserving content where possible.
    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        // Adjust existing lines width
        for line in &mut self.lines {
            if new_width > line.len() {
                line.resize_with(new_width, Cell::blank);
            } else {
                line.truncate(new_width);
            }
        }

        // Adjust number of lines
        if new_height > self.lines.len() {
            for _ in self.lines.len()..new_height {
                self.lines
                    .push((0..new_width).map(|_| Cell::blank()).collect());
            }
        } else {
            self.lines.truncate(new_height);
        }

        self.width = new_width;
        self.height = new_height;
    }

    /// Fill entire buffer with a cell.
    pub fn fill(&mut self, cell: &Cell) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.lines[y][x] = cell.clone();
            }
        }
    }

    /// Fill a rectangle with a cell.
    pub fn fill_rect(&mut self, cell: &Cell, rect: Rect) {
        let x_end = (rect.x as usize + rect.width as usize).min(self.width);
        let y_end = (rect.y as usize + rect.height as usize).min(self.height);
        for y in rect.y as usize..y_end {
            for x in rect.x as usize..x_end {
                self.lines[y][x] = cell.clone();
            }
        }
    }

    /// Clear the entire buffer (fill with blanks).
    pub fn clear(&mut self) {
        self.fill(&Cell::blank());
    }

    /// Clear a rectangular region.
    pub fn clear_rect(&mut self, rect: Rect) {
        self.fill_rect(&Cell::blank(), rect);
    }

    /// Insert `n` blank lines at position `y`, pushing existing lines down.
    /// Lines pushed past the bottom are discarded.
    pub fn insert_line(&mut self, y: usize, n: usize) {
        if y >= self.height {
            return;
        }
        for _ in 0..n {
            if self.lines.len() >= self.height {
                self.lines.pop();
            }
            let new_line = (0..self.width).map(|_| Cell::blank()).collect();
            self.lines.insert(y, new_line);
        }
    }

    /// Delete `n` lines at position `y`, pulling existing lines up.
    /// New blank lines are appended at the bottom.
    pub fn delete_line(&mut self, y: usize, n: usize) {
        if y >= self.height {
            return;
        }
        let count = n.min(self.height - y);
        self.lines.drain(y..y + count);
        for _ in 0..count {
            self.lines
                .push((0..self.width).map(|_| Cell::blank()).collect());
        }
    }

    /// Compute a hash for line `y` based on cell content (for diff algorithm).
    pub fn line_hash(&self, y: usize) -> u64 {
        if y >= self.height {
            return 0;
        }
        let line = &self.lines[y];
        let mut h: u64 = 0;
        for cell in line {
            h = h.wrapping_shl(5).wrapping_add(h).wrapping_add(cell.rune as u64);
        }
        h
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::style::CellStyle;

    #[test]
    fn test_new_buffer() {
        let buf = Buffer::new(80, 24);
        assert_eq!(buf.width(), 80);
        assert_eq!(buf.height(), 24);
        assert!(buf.cell(0, 0).unwrap().is_blank());
    }

    #[test]
    fn test_set_cell() {
        let mut buf = Buffer::new(10, 1);
        buf.set_cell(0, 0, Cell::new('A'));
        assert_eq!(buf.cell(0, 0).unwrap().rune, 'A');
    }

    #[test]
    fn test_wide_cell() {
        let mut buf = Buffer::new(10, 1);
        buf.set_cell(2, 0, Cell::new('中')); // width 2
        assert_eq!(buf.cell(2, 0).unwrap().rune, '中');
        assert_eq!(buf.cell(2, 0).unwrap().width, 2);
        assert!(buf.cell(3, 0).unwrap().is_empty()); // placeholder
    }

    #[test]
    fn test_overwrite_wide_cell() {
        let mut buf = Buffer::new(10, 1);
        buf.set_cell(2, 0, Cell::new('中')); // width 2 at [2,3]
        buf.set_cell(2, 0, Cell::new('A')); // overwrite at [2]
        assert_eq!(buf.cell(2, 0).unwrap().rune, 'A');
        assert_eq!(buf.cell(2, 0).unwrap().width, 1);
        // Position 3 should be blanked
        assert!(buf.cell(3, 0).unwrap().is_blank());
    }

    #[test]
    fn test_overwrite_wide_cell_partial() {
        let mut buf = Buffer::new(10, 1);
        buf.set_cell(2, 0, Cell::new('中')); // width 2 at [2,3]
        buf.set_cell(3, 0, Cell::new('B')); // overwrite placeholder at [3]
        // Both positions should be blanked/replaced
        assert!(buf.cell(2, 0).unwrap().is_blank());
        assert_eq!(buf.cell(3, 0).unwrap().rune, 'B');
    }

    #[test]
    fn test_resize() {
        let mut buf = Buffer::new(5, 5);
        buf.set_cell(0, 0, Cell::new('X'));
        buf.resize(10, 3);
        assert_eq!(buf.width(), 10);
        assert_eq!(buf.height(), 3);
        assert_eq!(buf.cell(0, 0).unwrap().rune, 'X');
    }

    #[test]
    fn test_insert_line() {
        let mut buf = Buffer::new(5, 3);
        buf.set_cell(0, 0, Cell::new('A'));
        buf.set_cell(0, 1, Cell::new('B'));
        buf.set_cell(0, 2, Cell::new('C'));
        buf.insert_line(1, 1);
        assert_eq!(buf.cell(0, 0).unwrap().rune, 'A');
        assert!(buf.cell(0, 1).unwrap().is_blank());
        assert_eq!(buf.cell(0, 2).unwrap().rune, 'B');
        // 'C' was pushed off the bottom
    }

    #[test]
    fn test_delete_line() {
        let mut buf = Buffer::new(5, 3);
        buf.set_cell(0, 0, Cell::new('A'));
        buf.set_cell(0, 1, Cell::new('B'));
        buf.set_cell(0, 2, Cell::new('C'));
        buf.delete_line(0, 1);
        assert_eq!(buf.cell(0, 0).unwrap().rune, 'B');
        assert_eq!(buf.cell(0, 1).unwrap().rune, 'C');
        assert!(buf.cell(0, 2).unwrap().is_blank());
    }

    #[test]
    fn test_line_hash() {
        let mut buf = Buffer::new(3, 2);
        buf.set_cell(0, 0, Cell::new('A'));
        buf.set_cell(1, 0, Cell::new('B'));
        let h1 = buf.line_hash(0);
        let h2 = buf.line_hash(1);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_styled_cell() {
        let mut style = CellStyle::new();
        style.fg = Some((255, 0, 0));
        let cell = Cell::new('X').with_style(style.clone());
        let mut buf = Buffer::new(5, 1);
        buf.set_cell(0, 0, cell);
        assert_eq!(buf.cell(0, 0).unwrap().style.fg, Some((255, 0, 0)));
    }
}
