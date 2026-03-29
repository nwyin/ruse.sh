//! A physics animation library providing damped spring and projectile motion
//! simulations.
//!
//! Ported from [charm.sh/harmonica](https://github.com/charmbracelet/harmonica).
//!
//! # Springs
//!
//! A damped harmonic oscillator that smoothly animates values toward a target:
//!
//! ```
//! use rouge_harmonica::{fps, Spring};
//!
//! let spring = Spring::new(fps(60), 6.0, 0.5);
//! let mut pos = 0.0;
//! let mut vel = 0.0;
//!
//! for _ in 0..600 {
//!     (pos, vel) = spring.update(pos, vel, 100.0);
//! }
//! assert!((pos - 100.0).abs() < 0.01);
//! ```
//!
//! # Projectiles
//!
//! Simple physics projectile motion with position, velocity, and acceleration:
//!
//! ```
//! use rouge_harmonica::{fps, Projectile, Point, GRAVITY};
//!
//! let mut proj = Projectile::new(
//!     fps(60),
//!     Point::new(0.0, 100.0, 0.0),
//!     Point::new(10.0, 0.0, 0.0),
//!     GRAVITY,
//! );
//!
//! for _ in 0..60 {
//!     proj.update();
//! }
//! ```

mod projectile;
mod spring;

pub use projectile::{Point, Projectile, Vector, GRAVITY, TERMINAL_GRAVITY};
pub use spring::Spring;

/// Convert frames-per-second to a delta time value (seconds per frame).
///
/// This is a convenience for computing the `delta_time` parameter when
/// creating springs or projectiles.
///
/// ```
/// use rouge_harmonica::fps;
///
/// let dt = fps(60);
/// assert!((dt - 1.0 / 60.0).abs() < f64::EPSILON);
/// ```
pub fn fps(n: u32) -> f64 {
    1.0 / n as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fps_60() {
        let dt = fps(60);
        assert!((dt - 1.0 / 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn fps_30() {
        let dt = fps(30);
        assert!((dt - 1.0 / 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn fps_1() {
        let dt = fps(1);
        assert!((dt - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn fps_120() {
        let dt = fps(120);
        assert!((dt - 1.0 / 120.0).abs() < f64::EPSILON);
    }
}
