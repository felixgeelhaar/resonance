//! Curve functions for mapping macro values to parameter ranges.
//!
//! Each curve maps an input in `[0.0, 1.0]` to an output in `[0.0, 1.0]`.
//! The result is then scaled to the mapping's target range.

use crate::dsl::ast::CurveKind;

/// Apply a curve function to a normalized value in `[0.0, 1.0]`.
///
/// Values are clamped to `[0.0, 1.0]` before applying the curve.
pub fn apply_curve(kind: CurveKind, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    match kind {
        CurveKind::Linear => t,
        CurveKind::Log => {
            // log10(1 + 9t) / log10(10) = log10(1 + 9t)
            (1.0 + 9.0 * t).log10()
        }
        CurveKind::Exp => t * t,
        CurveKind::Smoothstep => {
            // Hermite interpolation: 3t² - 2t³
            t * t * (3.0 - 2.0 * t)
        }
    }
}

/// Map a normalized macro value `[0.0, 1.0]` through a curve to a target range.
pub fn map_value(kind: CurveKind, t: f64, range: (f64, f64)) -> f64 {
    let curved = apply_curve(kind, t);
    range.0 + curved * (range.1 - range.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn linear_at_zero() {
        assert!((apply_curve(CurveKind::Linear, 0.0)).abs() < EPSILON);
    }

    #[test]
    fn linear_at_half() {
        assert!((apply_curve(CurveKind::Linear, 0.5) - 0.5).abs() < EPSILON);
    }

    #[test]
    fn linear_at_one() {
        assert!((apply_curve(CurveKind::Linear, 1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn exp_at_zero() {
        assert!((apply_curve(CurveKind::Exp, 0.0)).abs() < EPSILON);
    }

    #[test]
    fn exp_at_half() {
        assert!((apply_curve(CurveKind::Exp, 0.5) - 0.25).abs() < EPSILON);
    }

    #[test]
    fn exp_at_one() {
        assert!((apply_curve(CurveKind::Exp, 1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn log_at_zero() {
        assert!((apply_curve(CurveKind::Log, 0.0)).abs() < EPSILON);
    }

    #[test]
    fn log_at_half() {
        // log10(1 + 4.5) = log10(5.5) ≈ 0.7404
        let v = apply_curve(CurveKind::Log, 0.5);
        assert!(v > 0.5, "log curve should be above linear at 0.5, got {v}");
        assert!(v < 1.0);
    }

    #[test]
    fn log_at_one() {
        // log10(1 + 9) = log10(10) = 1.0
        assert!((apply_curve(CurveKind::Log, 1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn smoothstep_at_zero() {
        assert!((apply_curve(CurveKind::Smoothstep, 0.0)).abs() < EPSILON);
    }

    #[test]
    fn smoothstep_at_half() {
        // 3(0.25) - 2(0.125) = 0.75 - 0.25 = 0.5
        assert!((apply_curve(CurveKind::Smoothstep, 0.5) - 0.5).abs() < EPSILON);
    }

    #[test]
    fn smoothstep_at_one() {
        assert!((apply_curve(CurveKind::Smoothstep, 1.0) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn all_curves_monotonic() {
        for kind in [
            CurveKind::Linear,
            CurveKind::Log,
            CurveKind::Exp,
            CurveKind::Smoothstep,
        ] {
            let mut prev = apply_curve(kind, 0.0);
            for i in 1..=100 {
                let t = i as f64 / 100.0;
                let v = apply_curve(kind, t);
                assert!(
                    v >= prev - EPSILON,
                    "{kind:?} not monotonic at t={t}: {prev} > {v}"
                );
                prev = v;
            }
        }
    }

    #[test]
    fn clamp_below_zero() {
        assert!((apply_curve(CurveKind::Linear, -0.5)).abs() < EPSILON);
    }

    #[test]
    fn clamp_above_one() {
        assert!((apply_curve(CurveKind::Linear, 1.5) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn map_value_scales_to_range() {
        let v = map_value(CurveKind::Linear, 0.5, (100.0, 200.0));
        assert!((v - 150.0).abs() < EPSILON);
    }

    #[test]
    fn map_value_at_zero() {
        let v = map_value(CurveKind::Exp, 0.0, (20.0, 20000.0));
        assert!((v - 20.0).abs() < EPSILON);
    }

    #[test]
    fn map_value_at_one() {
        let v = map_value(CurveKind::Exp, 1.0, (20.0, 20000.0));
        assert!((v - 20000.0).abs() < EPSILON);
    }

    #[test]
    fn map_value_with_log_curve() {
        let v = map_value(CurveKind::Log, 0.5, (0.0, 1000.0));
        // Should be above 500 due to log curve
        assert!(v > 500.0, "log mapped value should be > 500, got {v}");
    }
}
