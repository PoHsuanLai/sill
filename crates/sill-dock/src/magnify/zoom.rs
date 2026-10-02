//! design/10 §10.5.1: `zoom_i = 1 + (M - 1)(1 - q_i^2) P`, `q_i = min(|x - c_i|, R) / R`.
//!
//! The appendix transcribes Plank as `1 + (1 - q^2)(M P - 1)`, which shrinks tiles at `P = 0`;
//! the doc's form (used here) gives exactly 1 at `P = 0`.

use crate::magnify::MagnifyParams;
use sill_model::units::{Cpx, Permille};

/// The zoom of the tile centred at `center` for pointer `x` at progress `progress`, in
/// thousandths (1000 = rest).
pub fn zoom(center: Cpx, x: Cpx, progress: Permille, p: &MagnifyParams) -> Permille {
    Permille((factor(center, x, progress, p) * 1000.0).round() as u32)
}

/// Every tile's magnified size.
pub fn sizes(centers: &[Cpx], x: Cpx, progress: Permille, p: &MagnifyParams) -> Vec<Cpx> {
    centers
        .iter()
        .map(|&c| Cpx((f64::from(p.tile.0) * factor(c, x, progress, p)).round() as i32))
        .collect()
}

/// The unrounded zoom factor.
pub(crate) fn factor(center: Cpx, x: Cpx, progress: Permille, p: &MagnifyParams) -> f64 {
    let tile = f64::from(p.tile.0);
    let radius = f64::from(p.radius.0);
    if tile <= 0.0 || radius <= 0.0 {
        return 1.0;
    }
    let max = f64::from(p.magnified.0) / tile;
    let q = (f64::from(x.0 - center.0).abs()).min(radius) / radius;
    let progress = f64::from(progress.0.min(1000)) / 1000.0;
    1.0 + (max - 1.0) * (1.0 - q * q) * progress
}

#[cfg(test)]
mod tests {
    use super::{sizes, zoom};
    use crate::magnify::MagnifyParams;
    use sill_model::units::{Cpx, Permille};

    /// The doubled tile (S = 96, R = 96, M = 2), the worked numbers of design/10 §10.5 and §10.8;
    /// the settings' own default is the 72 px cap.
    fn twice(output: Cpx) -> MagnifyParams {
        MagnifyParams {
            magnified: Cpx::px(96),
            radius: Cpx::px(96),
            ..MagnifyParams::design_defaults(output)
        }
    }

    fn p() -> MagnifyParams {
        twice(Cpx::px(1920))
    }

    /// Seven tiles at pitch `T + g` = 52 px.
    fn centers() -> Vec<Cpx> {
        (0..7).map(|k| Cpx::px(100 + 52 * k)).collect()
    }

    #[test]
    fn the_design_curve_table_holds_to_a_tenth_of_a_pixel() {
        // design/10 §10.8 test 1: P = 1, pointer on tile 3 of 7.
        let got = sizes(&centers(), Cpx::px(100 + 52 * 3), Permille::ONE, &p());
        let want = [4800, 4800, 8192, 9600, 8192, 4800, 4800];
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            assert!((g.0 - w).abs() <= 10, "tile {i}: {} vs {w}", g.0);
        }
        // §10.5.1's worked extra width: 115.8 px.
        let extra: i32 = got.iter().map(|s| s.0 - 4800).sum();
        assert!((extra - 11_583).abs() <= 10, "extra {extra}");
    }

    #[test]
    fn progress_zero_is_exactly_rest_everywhere() {
        for x in [0, 100, 178, 256, 9999] {
            let got = sizes(&centers(), Cpx::px(x), Permille::ZERO, &p());
            assert!(got.iter().all(|s| *s == Cpx::px(48)), "x {x}: {got:?}");
        }
    }

    #[test]
    fn zoom_falls_with_distance_to_one_at_the_radius() {
        let c = Cpx::px(500);
        const CASES: &[(i32, u32)] = &[
            (0, 2000),
            (24, 1938),
            (48, 1750),
            (52, 1707),
            (96, 1000),
            (200, 1000),
            (-52, 1707),
        ];
        for &(d, want) in CASES {
            assert_eq!(
                zoom(c, Cpx::px(500 + d), Permille::ONE, &p()),
                Permille(want),
                "d {d}"
            );
        }
        // Half progress is half the extra zoom.
        assert_eq!(zoom(c, c, Permille(500), &p()), Permille(1500));
    }

    #[test]
    fn a_wider_radius_reaches_the_second_neighbour() {
        // design/10 §10.9 open decision 1: R = 144 affects two neighbours each side.
        let wide = MagnifyParams {
            radius: Cpx::px(144),
            ..p()
        };
        let got = sizes(&centers(), Cpx::px(100 + 52 * 3), Permille::ONE, &wide);
        assert!(got[1].0 > 4800 && got[5].0 > 4800, "{got:?}");
        assert_eq!(got[0], Cpx::px(48));
    }
}
