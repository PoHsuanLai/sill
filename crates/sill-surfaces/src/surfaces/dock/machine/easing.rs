//! Evaluating a CSS `cubic-bezier` at a point in time, for motion the shell drives from a frame
//! clock rather than from CSS: the dock's magnification progress (design/10 §10.5.4) and its
//! auto-hide slide (§10.5.5). quire's `ds::CubicBezier` is token data with no evaluator; its
//! control points in thousandths are what [`ease`] takes.

use sill_services::Permille;

/// The curve with control points `[x1, y1, x2, y2]` (thousandths, as `ds::CubicBezier.0`) at
/// time `t` (thousandths, clamped to 0..=1000). The result is in thousandths; a spring's
/// overshoot exceeds 1000, and a value below 0 is clamped to 0.
pub(super) fn ease(control: [i16; 4], t: Permille) -> Permille {
    let [x1, y1, x2, y2] = control.map(|c| f64::from(c) / 1000.0);
    let t = f64::from(t.0.min(1000)) / 1000.0;
    let s = solve_x(x1, x2, t);
    let y = bezier(y1, y2, s);
    Permille((y * 1000.0).round().max(0.0) as u32)
}

/// One coordinate of a unit cubic Bézier with inner control values `p1`, `p2` at parameter `s`.
fn bezier(p1: f64, p2: f64, s: f64) -> f64 {
    let u = 1.0 - s;
    3.0 * u * u * s * p1 + 3.0 * u * s * s * p2 + s * s * s
}

/// The parameter `s` whose x is `x`: Newton's method from `x`, bisection when it stalls. CSS
/// requires `x1, x2` in 0..=1, which makes x monotonic in `s`.
fn solve_x(x1: f64, x2: f64, x: f64) -> f64 {
    let mut s = x;
    for _ in 0..8 {
        let err = bezier(x1, x2, s) - x;
        if err.abs() < 1e-7 {
            return s;
        }
        let u = 1.0 - s;
        let slope = 3.0 * u * u * x1 + 6.0 * u * s * (x2 - x1) + 3.0 * s * s * (1.0 - x2);
        if slope.abs() < 1e-6 {
            break;
        }
        s = (s - err / slope).clamp(0.0, 1.0);
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..40 {
        s = (lo + hi) / 2.0;
        if bezier(x1, x2, s) < x {
            lo = s;
        } else {
            hi = s;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::ease;
    use sill_services::Permille;

    /// `--e-out` as design/10 §10.5.4 names it: `bezier(.22, .9, .30, 1)`.
    const OUT: [i16; 4] = [220, 900, 300, 1000];
    const LINEAR: [i16; 4] = [0, 0, 1000, 1000];

    #[test]
    fn ends_are_exact_and_linear_is_identity() {
        for curve in [OUT, LINEAR, [550, 0, 750, 200], [340, 1420, 520, 1000]] {
            assert_eq!(ease(curve, Permille(0)), Permille(0), "{curve:?} at 0");
            assert_eq!(
                ease(curve, Permille(1000)),
                Permille(1000),
                "{curve:?} at 1"
            );
        }
        for t in [0, 100, 250, 500, 999] {
            assert_eq!(ease(LINEAR, Permille(t)), Permille(t), "linear at {t}");
        }
    }

    #[test]
    fn known_values_of_ease_out() {
        // cubic-bezier(.22, .9, .30, 1) by plain bisection in Python, to 0.01 thousandths.
        const CASES: &[(u32, u32)] = &[(100, 380), (250, 748), (500, 949), (750, 993)];
        for &(t, want) in CASES {
            let got = ease(OUT, Permille(t)).0;
            assert!(got.abs_diff(want) <= 1, "t {t}: got {got}, want {want}");
        }
    }

    #[test]
    fn a_spring_overshoots_and_time_past_the_end_clamps() {
        let spring = [340, 1420, 520, 1000];
        let peak = (0..=1000)
            .step_by(10)
            .map(|t| ease(spring, Permille(t)).0)
            .max();
        assert!(peak.is_some_and(|p| p > 1000), "{peak:?}");
        assert_eq!(ease(OUT, Permille(4000)), Permille(1000));
    }
}
