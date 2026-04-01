/// A position in a cell grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Position {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

/// A rectangle in a cell grid (exclusive max).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, px: u16, py: u16) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    pub fn right(&self) -> u16 {
        self.x + self.width
    }

    pub fn bottom(&self) -> u16 {
        self.y + self.height
    }

    /// Create a Rect covering the full terminal area.
    pub fn full(width: u16, height: u16) -> Self {
        Self::new(0, 0, width, height)
    }

    /// Split into left and right panes. `left_width` columns go to the left.
    pub fn split_horizontal(self, left_width: u16) -> (Rect, Rect) {
        let lw = left_width.min(self.width);
        let left = Rect::new(self.x, self.y, lw, self.height);
        let right = Rect::new(self.x + lw, self.y, self.width - lw, self.height);
        (left, right)
    }

    /// Split into top and bottom panes. `top_height` rows go to the top.
    pub fn split_vertical(self, top_height: u16) -> (Rect, Rect) {
        let th = top_height.min(self.height);
        let top = Rect::new(self.x, self.y, self.width, th);
        let bottom = Rect::new(self.x, self.y + th, self.width, self.height - th);
        (top, bottom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_horizontal() {
        let r = Rect::new(0, 0, 80, 24);
        let (left, right) = r.split_horizontal(30);
        assert_eq!(left, Rect::new(0, 0, 30, 24));
        assert_eq!(right, Rect::new(30, 0, 50, 24));
    }

    #[test]
    fn test_split_vertical() {
        let r = Rect::new(0, 0, 80, 24);
        let (top, bottom) = r.split_vertical(20);
        assert_eq!(top, Rect::new(0, 0, 80, 20));
        assert_eq!(bottom, Rect::new(0, 20, 80, 4));
    }

    #[test]
    fn test_split_horizontal_offset() {
        let r = Rect::new(10, 5, 60, 20);
        let (left, right) = r.split_horizontal(25);
        assert_eq!(left, Rect::new(10, 5, 25, 20));
        assert_eq!(right, Rect::new(35, 5, 35, 20));
    }

    #[test]
    fn test_split_clamps() {
        let r = Rect::new(0, 0, 10, 10);
        let (left, right) = r.split_horizontal(100);
        assert_eq!(left, Rect::new(0, 0, 10, 10));
        assert_eq!(right, Rect::new(10, 0, 0, 10));
    }

    #[test]
    fn test_full() {
        let r = Rect::full(80, 24);
        assert_eq!(r, Rect::new(0, 0, 80, 24));
    }
}
