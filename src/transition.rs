use std::time::Duration;

/// Easing used by the upstream renderer's transitions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Easing {
    Linear,
    #[default]
    CubicOut,
    ElasticOut,
    /// CSS `ease`: cubic-bezier(0.25, 0.1, 0.25, 1).
    CssEase,
    /// CSS cubic Bézier control points `[x1, y1, x2, y2]`.
    /// Both x coordinates must lie in `0..=1`; y may overshoot.
    CubicBezier([f64; 4]),
}

/// A renderer-independent transition description. The host advances the clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub duration: Duration,
    pub easing: Easing,
}

impl Default for Transition {
    fn default() -> Self {
        Self {
            duration: Duration::from_millis(200),
            easing: Easing::CubicOut,
        }
    }
}

impl Transition {
    /// Evaluate the upstream cubic-out or d3-ease 3.0.1 elastic-out curve.
    /// Elastic easing deliberately overshoots; renderers must not clamp it.
    pub fn progress(self, elapsed: Duration) -> f64 {
        if self.duration.is_zero() || elapsed >= self.duration {
            return 1.0;
        }
        let t = elapsed.as_secs_f64() / self.duration.as_secs_f64();
        match self.easing {
            Easing::Linear => t,
            Easing::CubicOut => 1.0 - (1.0 - t).powi(3),
            Easing::ElasticOut => {
                let decay = ((-10.0 * t).exp2() - 1.0 / 1024.0) / (1.0 - 1.0 / 1024.0);
                1.0 - decay * ((t + 0.075) * std::f64::consts::TAU / 0.3).sin()
            }
            Easing::CssEase | Easing::CubicBezier(_) => {
                let [x1, y1, x2, y2] = match self.easing {
                    Easing::CubicBezier(points) => points,
                    _ => [0.25, 0.1, 0.25, 1.],
                };
                // Invert the Bézier's x coordinate before evaluating its y.
                let coordinate = |u: f64, a: f64, b: f64| {
                    3. * (1. - u).powi(2) * u * a + 3. * (1. - u) * u * u * b + u.powi(3)
                };
                let (mut lo, mut hi) = (0., 1.);
                for _ in 0..48 {
                    let mid = (lo + hi) * 0.5;
                    if coordinate(mid, x1, x2) < t {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                coordinate((lo + hi) * 0.5, y1, y2)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubic_bezier_inverts_x_instead_of_treating_time_as_curve_parameter() {
        let transition = Transition {
            duration: Duration::from_secs(1),
            easing: Easing::CubicBezier([0.2, 0.8, 0.2, 1.0]),
        };
        // At curve parameter u=1/2, the source CSS curve has x=.275, y=.8.
        assert!((transition.progress(Duration::from_millis(275)) - 0.8).abs() < 1e-12);
        assert_eq!(transition.progress(Duration::from_secs(1)), 1.);
        for t in [0.05, 0.25, 0.5, 0.9] {
            let linear = Transition {
                easing: Easing::CubicBezier([0.1, 0.1, 0.7, 0.7]),
                ..transition
            };
            assert!((linear.progress(Duration::from_secs_f64(t)) - t).abs() < 1e-12);
        }
    }
}
