// This file defines a simplified damped harmonic oscillator, colloquially
// known as a spring. Ported from Ryan Juckett's simple damped harmonic
// motion, originally written in C++, via the Go port by Charmbracelet, Inc.
//
// For background on the algorithm see:
// https://www.ryanjuckett.com/damped-springs/

/******************************************************************************

  Copyright (c) 2008-2012 Ryan Juckett
  http://www.ryanjuckett.com/

  This software is provided 'as-is', without any express or implied
  warranty. In no event will the authors be held liable for any damages
  arising from the use of this software.

  Permission is granted to anyone to use this software for any purpose,
  including commercial applications, and to alter it and redistribute it
  freely, subject to the following restrictions:

  1. The origin of this software must not be misrepresented; you must not
     claim that you wrote the original software. If you use this software
     in a product, an acknowledgment in the product documentation would be
     appreciated but is not required.

  2. Altered source versions must be plainly marked as such, and must not be
     misrepresented as being the original software.

  3. This notice may not be removed or altered from any source
     distribution.

*******************************************************************************

  Ported to Go by Charmbracelet, Inc. in 2021.
  Ported to Rust by rouge.sh.

******************************************************************************/

/// Machine epsilon: the smallest value such that 1.0 + EPSILON != 1.0.
const EPSILON: f64 = f64::EPSILON;

/// A damped harmonic oscillator (spring) with pre-computed coefficients for
/// efficient per-frame updates.
///
/// Create a `Spring` once with your desired parameters, then call
/// [`Spring::update`] each frame to animate values toward a target.
///
/// # Example
///
/// ```
/// use rouge_harmonica::{fps, Spring};
///
/// let spring = Spring::new(fps(60), 6.0, 0.2);
///
/// let mut pos = 0.0;
/// let mut vel = 0.0;
/// let target = 100.0;
///
/// for _ in 0..600 {
///     (pos, vel) = spring.update(pos, vel, target);
/// }
///
/// assert!((pos - target).abs() < 0.01);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pos_pos_coef: f64,
    pos_vel_coef: f64,
    vel_pos_coef: f64,
    vel_vel_coef: f64,
}

impl Spring {
    /// Create a new spring, pre-computing motion coefficients.
    ///
    /// - `delta_time`: time step per frame (use [`fps`](crate::fps) helper).
    /// - `angular_frequency`: angular frequency of motion; affects speed.
    /// - `damping_ratio`: determines oscillation behavior:
    ///   - `> 1.0`: over-damped (no oscillation, slow convergence)
    ///   - `= 1.0`: critically damped (fastest non-oscillatory convergence)
    ///   - `< 1.0`: under-damped (oscillates with decaying amplitude)
    pub fn new(delta_time: f64, angular_frequency: f64, damping_ratio: f64) -> Self {
        let angular_frequency = angular_frequency.max(0.0);
        let damping_ratio = damping_ratio.max(0.0);

        // If there is no angular frequency, the spring will not move;
        // return identity coefficients.
        if angular_frequency < EPSILON {
            return Self {
                pos_pos_coef: 1.0,
                pos_vel_coef: 0.0,
                vel_pos_coef: 0.0,
                vel_vel_coef: 1.0,
            };
        }

        if damping_ratio > 1.0 + EPSILON {
            // Over-damped.
            let za = -angular_frequency * damping_ratio;
            let zb = angular_frequency * (damping_ratio * damping_ratio - 1.0).sqrt();
            let z1 = za - zb;
            let z2 = za + zb;

            let e1 = (z1 * delta_time).exp();
            let e2 = (z2 * delta_time).exp();

            let inv_two_zb = 1.0 / (2.0 * zb); // = 1 / (z2 - z1)

            let e1_over_two_zb = e1 * inv_two_zb;
            let e2_over_two_zb = e2 * inv_two_zb;

            let z1e1_over_two_zb = z1 * e1_over_two_zb;
            let z2e2_over_two_zb = z2 * e2_over_two_zb;

            Self {
                pos_pos_coef: e1_over_two_zb * z2 - z2e2_over_two_zb + e2,
                pos_vel_coef: -e1_over_two_zb + e2_over_two_zb,
                vel_pos_coef: (z1e1_over_two_zb - z2e2_over_two_zb + e2) * z2,
                vel_vel_coef: -z1e1_over_two_zb + z2e2_over_two_zb,
            }
        } else if damping_ratio < 1.0 - EPSILON {
            // Under-damped.
            let omega_zeta = angular_frequency * damping_ratio;
            let alpha = angular_frequency * (1.0 - damping_ratio * damping_ratio).sqrt();

            let exp_term = (-omega_zeta * delta_time).exp();
            let cos_term = (alpha * delta_time).cos();
            let sin_term = (alpha * delta_time).sin();

            let inv_alpha = 1.0 / alpha;

            let exp_sin = exp_term * sin_term;
            let exp_cos = exp_term * cos_term;
            let exp_omega_zeta_sin_over_alpha = exp_term * omega_zeta * sin_term * inv_alpha;

            Self {
                pos_pos_coef: exp_cos + exp_omega_zeta_sin_over_alpha,
                pos_vel_coef: exp_sin * inv_alpha,
                vel_pos_coef: -exp_sin * alpha - omega_zeta * exp_omega_zeta_sin_over_alpha,
                vel_vel_coef: exp_cos - exp_omega_zeta_sin_over_alpha,
            }
        } else {
            // Critically damped.
            let exp_term = (-angular_frequency * delta_time).exp();
            let time_exp = delta_time * exp_term;
            let time_exp_freq = time_exp * angular_frequency;

            Self {
                pos_pos_coef: time_exp_freq + exp_term,
                pos_vel_coef: time_exp,
                vel_pos_coef: -angular_frequency * time_exp_freq,
                vel_vel_coef: -time_exp_freq + exp_term,
            }
        }
    }

    /// Update position and velocity values toward a target (equilibrium)
    /// position. Returns `(new_position, new_velocity)`.
    pub fn update(&self, pos: f64, vel: f64, target: f64) -> (f64, f64) {
        let old_pos = pos - target; // work in equilibrium-relative space
        let old_vel = vel;

        let new_pos = old_pos * self.pos_pos_coef + old_vel * self.pos_vel_coef + target;
        let new_vel = old_pos * self.vel_pos_coef + old_vel * self.vel_vel_coef;

        (new_pos, new_vel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fps;

    const TOLERANCE: f64 = 0.01;

    fn run_spring(spring: Spring, target: f64, steps: usize) -> (f64, f64) {
        let mut pos = 0.0;
        let mut vel = 0.0;
        for _ in 0..steps {
            (pos, vel) = spring.update(pos, vel, target);
        }
        (pos, vel)
    }

    #[test]
    fn under_damped_converges() {
        let spring = Spring::new(fps(60), 6.0, 0.2);
        let (pos, vel) = run_spring(spring, 100.0, 600);
        assert!(
            (pos - 100.0).abs() < TOLERANCE,
            "under-damped: pos={pos}, expected ~100.0"
        );
        assert!(
            vel.abs() < TOLERANCE,
            "under-damped: vel={vel}, expected ~0.0"
        );
    }

    #[test]
    fn critically_damped_converges() {
        let spring = Spring::new(fps(60), 6.0, 1.0);
        let (pos, vel) = run_spring(spring, 100.0, 600);
        assert!(
            (pos - 100.0).abs() < TOLERANCE,
            "critically-damped: pos={pos}, expected ~100.0"
        );
        assert!(
            vel.abs() < TOLERANCE,
            "critically-damped: vel={vel}, expected ~0.0"
        );
    }

    #[test]
    fn over_damped_converges() {
        let spring = Spring::new(fps(60), 6.0, 2.0);
        let (pos, vel) = run_spring(spring, 100.0, 600);
        assert!(
            (pos - 100.0).abs() < TOLERANCE,
            "over-damped: pos={pos}, expected ~100.0"
        );
        assert!(
            vel.abs() < TOLERANCE,
            "over-damped: vel={vel}, expected ~0.0"
        );
    }

    #[test]
    fn zero_frequency_is_identity() {
        let spring = Spring::new(fps(60), 0.0, 1.0);
        let (pos, vel) = spring.update(5.0, 3.0, 100.0);
        assert!(
            (pos - 5.0).abs() < f64::EPSILON,
            "zero freq should not move position"
        );
        assert!(
            (vel - 3.0).abs() < f64::EPSILON,
            "zero freq should not change velocity"
        );
    }

    #[test]
    fn negative_target() {
        let spring = Spring::new(fps(60), 6.0, 0.5);
        let (pos, _) = run_spring(spring, -50.0, 600);
        assert!(
            (pos - (-50.0)).abs() < TOLERANCE,
            "negative target: pos={pos}, expected ~-50.0"
        );
    }

    #[test]
    fn high_angular_frequency() {
        let spring = Spring::new(fps(60), 20.0, 0.5);
        let (pos, _) = run_spring(spring, 100.0, 600);
        assert!(
            (pos - 100.0).abs() < TOLERANCE,
            "high frequency: pos={pos}, expected ~100.0"
        );
    }

    #[test]
    fn low_angular_frequency() {
        let spring = Spring::new(fps(60), 1.0, 0.5);
        // Low frequency takes longer to converge; give it more steps.
        let (pos, _) = run_spring(spring, 100.0, 3000);
        assert!(
            (pos - 100.0).abs() < TOLERANCE,
            "low frequency: pos={pos}, expected ~100.0"
        );
    }

    #[test]
    fn under_damped_overshoots() {
        let spring = Spring::new(fps(60), 6.0, 0.1);
        let mut pos = 0.0;
        let mut vel = 0.0;
        let target = 100.0;
        let mut max_pos = 0.0_f64;
        for _ in 0..600 {
            (pos, vel) = spring.update(pos, vel, target);
            max_pos = max_pos.max(pos);
        }
        assert!(
            max_pos > target,
            "under-damped spring should overshoot: max_pos={max_pos}"
        );
    }
}
