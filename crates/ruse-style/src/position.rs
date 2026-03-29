/// A position along a horizontal or vertical axis.
///
/// 0.0 = start (left/top), 1.0 = end (right/bottom), 0.5 = center.
#[derive(Clone, Copy, PartialEq, Default)]
pub struct Position(f64);

impl Position {
    pub const TOP: Self = Self(0.0);
    pub const BOTTOM: Self = Self(1.0);
    pub const LEFT: Self = Self(0.0);
    pub const RIGHT: Self = Self(1.0);
    pub const CENTER: Self = Self(0.5);

    pub fn new(v: f64) -> Self {
        Self(v.clamp(0.0, 1.0))
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

impl std::fmt::Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Position({:.2})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp() {
        assert_eq!(Position::new(1.5).value(), 1.0);
        assert_eq!(Position::new(-0.5).value(), 0.0);
        assert_eq!(Position::new(0.3).value(), 0.3);
    }

    #[test]
    fn test_constants() {
        assert_eq!(Position::TOP.value(), 0.0);
        assert_eq!(Position::BOTTOM.value(), 1.0);
        assert_eq!(Position::CENTER.value(), 0.5);
        assert_eq!(Position::LEFT.value(), 0.0);
        assert_eq!(Position::RIGHT.value(), 1.0);
    }
}
