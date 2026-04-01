// Simple physics projectile motion simulation.
//
// For background on projectile motion see:
// https://en.wikipedia.org/wiki/Projectile_motion

/// A point in 3D space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point {
    /// Create a new point.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// A vector carrying magnitude and direction, represented as a point from the
/// origin where the magnitude is the euclidean distance and the direction is
/// the direction to the point from the origin.
pub type Vector = Point;

/// Gravity vector for coordinate systems with the origin in the bottom-left
/// corner:
///
/// ```text
///   y             y ±z
///   │             │ /
///   │             │/
///   └───── ±x     └───── ±x
/// ```
pub const GRAVITY: Vector = Point {
    x: 0.0,
    y: -9.81,
    z: 0.0,
};

/// Gravity vector for coordinate systems with the origin in the top-left
/// corner (e.g. terminal UIs where Y increases downward).
pub const TERMINAL_GRAVITY: Vector = Point {
    x: 0.0,
    y: 9.81,
    z: 0.0,
};

/// A projectile with position, velocity, and acceleration on a 3D plane.
///
/// # Example
///
/// ```
/// use ruse_harmonica::{fps, Projectile, Point, TERMINAL_GRAVITY};
///
/// let mut proj = Projectile::new(
///     fps(60),
///     Point::new(6.0, 100.0, 0.0),
///     Point::new(2.0, 0.0, 0.0),
///     TERMINAL_GRAVITY,
/// );
///
/// for _ in 0..60 {
///     proj.update();
/// }
///
/// let pos = proj.position();
/// assert!(pos.x > 6.0); // moved right
/// assert!(pos.y > 100.0); // fell down (terminal coords)
/// ```
pub struct Projectile {
    pos: Point,
    vel: Vector,
    acc: Vector,
    delta_time: f64,
}

impl Projectile {
    /// Create a new projectile.
    ///
    /// - `delta_time`: time step per frame (use [`fps`](crate::fps)).
    /// - `position`: initial position.
    /// - `velocity`: initial velocity.
    /// - `acceleration`: constant acceleration (e.g. gravity).
    pub fn new(delta_time: f64, position: Point, velocity: Vector, acceleration: Vector) -> Self {
        Self {
            pos: position,
            vel: velocity,
            acc: acceleration,
            delta_time,
        }
    }

    /// Advance the projectile by one time step, updating position and velocity.
    /// Returns the new position.
    pub fn update(&mut self) -> Point {
        self.pos.x += self.vel.x * self.delta_time;
        self.pos.y += self.vel.y * self.delta_time;
        self.pos.z += self.vel.z * self.delta_time;

        self.vel.x += self.acc.x * self.delta_time;
        self.vel.y += self.acc.y * self.delta_time;
        self.vel.z += self.acc.z * self.delta_time;

        self.pos
    }

    /// Returns the current position.
    pub fn position(&self) -> Point {
        self.pos
    }

    /// Returns the current velocity.
    pub fn velocity(&self) -> Vector {
        self.vel
    }

    /// Returns the current acceleration.
    pub fn acceleration(&self) -> Vector {
        self.acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fps;

    const TOLERANCE: f64 = 0.1;

    #[test]
    fn projectile_moves_under_gravity() {
        let dt = fps(60);
        let mut proj = Projectile::new(
            dt,
            Point::new(0.0, 100.0, 0.0),
            Point::new(10.0, 0.0, 0.0),
            GRAVITY,
        );

        for _ in 0..60 {
            proj.update();
        }

        let pos = proj.position();
        // After 1 second at 10 m/s horizontal, x should be ~10.
        assert!(
            (pos.x - 10.0).abs() < TOLERANCE,
            "x={}, expected ~10.0",
            pos.x
        );
        // After 1 second under gravity from y=100, y = 100 + 0 - 0.5*9.81*1^2 ~ 95.095
        assert!(
            (pos.y - 95.095).abs() < 1.0,
            "y={}, expected ~95.095",
            pos.y
        );
    }

    #[test]
    fn projectile_terminal_gravity() {
        let dt = fps(60);
        let mut proj = Projectile::new(
            dt,
            Point::new(0.0, 0.0, 0.0),
            Point::new(0.0, 0.0, 0.0),
            TERMINAL_GRAVITY,
        );

        for _ in 0..60 {
            proj.update();
        }

        let pos = proj.position();
        // Under terminal gravity (positive y), y should increase.
        assert!(
            pos.y > 0.0,
            "y={}, expected > 0.0 under terminal gravity",
            pos.y
        );
    }

    #[test]
    fn projectile_no_acceleration() {
        let dt = fps(60);
        let mut proj = Projectile::new(
            dt,
            Point::new(0.0, 0.0, 0.0),
            Point::new(5.0, 3.0, -1.0),
            Point::new(0.0, 0.0, 0.0),
        );

        for _ in 0..60 {
            proj.update();
        }

        let pos = proj.position();
        // After 1 second of constant velocity.
        assert!(
            (pos.x - 5.0).abs() < TOLERANCE,
            "x={}, expected ~5.0",
            pos.x
        );
        assert!(
            (pos.y - 3.0).abs() < TOLERANCE,
            "y={}, expected ~3.0",
            pos.y
        );
        assert!(
            (pos.z - (-1.0)).abs() < TOLERANCE,
            "z={}, expected ~-1.0",
            pos.z
        );
    }

    #[test]
    fn projectile_accessors() {
        let proj = Projectile::new(
            fps(60),
            Point::new(1.0, 2.0, 3.0),
            Point::new(4.0, 5.0, 6.0),
            Point::new(7.0, 8.0, 9.0),
        );

        assert_eq!(proj.position(), Point::new(1.0, 2.0, 3.0));
        assert_eq!(proj.velocity(), Point::new(4.0, 5.0, 6.0));
        assert_eq!(proj.acceleration(), Point::new(7.0, 8.0, 9.0));
    }

    #[test]
    fn projectile_3d_motion() {
        let dt = fps(60);
        let mut proj = Projectile::new(
            dt,
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 2.0, 3.0),
            Point::new(0.0, 0.0, 0.0),
        );

        let pos = proj.update();
        assert!((pos.x - dt).abs() < 1e-10);
        assert!((pos.y - 2.0 * dt).abs() < 1e-10);
        assert!((pos.z - 3.0 * dt).abs() < 1e-10);
    }
}
