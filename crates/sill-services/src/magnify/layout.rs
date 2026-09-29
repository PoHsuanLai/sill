//! design/10 §10.5.2: the pointer-anchored layout. The slot under the pointer keeps the
//! pointer at the same fraction of its width, neighbours are chained outward from it, and the
//! pill is clamped inside the output.
//!
//! One deliberate difference from the doc: the doc extends the outer slots to the pill's padding
//! edge, which makes the layout at `P = 0` move with the pointer inside those
//! slots. Here every slot is `T + g` wide and a pointer beyond the outer slots is clamped to
//! their edge, so rest is rest wherever the pointer is. Gaps between slots wider than `g` (the
//! Trash separator block, §10.3.9) keep their rest width and follow the pointer through them.

use serde::{Deserialize, Serialize};

use crate::magnify::MagnifyParams;
use crate::magnify::zoom::factor;
use sill_model::units::{Cpx, Permille};

/// One tile's magnified box: left edge and size (it is square, bottom on the baseline).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileBox {
    pub left: Cpx,
    pub size: Cpx,
}

/// A frame's dock geometry, in surface x coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    pub tiles: Vec<TileBox>,
    pub pill_left: Cpx,
    pub pill_right: Cpx,
}

/// The rest centres of `count` tiles in a pill centred on the output (clamped to the edge
/// distance when the pill is wider than the output allows).
pub fn rest_centers(count: usize, p: &MagnifyParams) -> Vec<Cpx> {
    let n = i32::try_from(count).unwrap_or(i32::MAX);
    if n == 0 {
        return Vec::new();
    }
    let pitch = p.tile.0 + p.gap.0;
    let width = n * p.tile.0 + (n - 1) * p.gap.0 + 2 * p.pad.0;
    let left = ((p.output_width.0 - width) / 2).max(p.edge_clamp.0);
    (0..n)
        .map(|k| Cpx(left + p.pad.0 + p.tile.0 / 2 + k * pitch))
        .collect()
}

/// Where the pointer sits: in a slot at a fraction of its width, or in a wider gap between two
/// slots (which keeps its rest position).
enum Anchor {
    Slot { index: usize, fraction: f64 },
    GapAfter { index: usize },
}

/// The layout for rest centres `centers`, pointer `pointer_x` and progress `progress`.
pub fn magnify(centers: &[Cpx], pointer_x: Cpx, progress: Permille, p: &MagnifyParams) -> Layout {
    let Some((&first, &last)) = centers.first().zip(centers.last()) else {
        let middle = Cpx(p.output_width.0 / 2);
        return Layout {
            tiles: Vec::new(),
            pill_left: middle,
            pill_right: middle,
        };
    };
    let tile = f64::from(p.tile.0);
    let gap = f64::from(p.gap.0);
    let half = (tile + gap) / 2.0;
    let rest_left: Vec<f64> = centers.iter().map(|c| f64::from(c.0) - half).collect();
    let x = f64::from(pointer_x.0).clamp(f64::from(first.0) - half, f64::from(last.0) + half);
    let widths: Vec<f64> = centers
        .iter()
        .map(|&c| tile * factor(c, pointer_x, progress, p) + gap)
        .collect();

    let left = chain(
        &rest_left,
        &widths,
        anchor(&rest_left, x, tile + gap),
        x,
        tile + gap,
    );
    let n = centers.len();
    let pill_left = left[0] - f64::from(p.pad.0) + gap / 2.0;
    let pill_right = left[n - 1] + widths[n - 1] + f64::from(p.pad.0) - gap / 2.0;
    let shift = clamp_shift(pill_left, pill_right, p);

    let round = |v: f64| Cpx((v + shift).round() as i32);
    Layout {
        tiles: left
            .iter()
            .zip(&widths)
            .map(|(&l, &w)| TileBox {
                left: round(l + gap / 2.0),
                size: Cpx((w - gap).round() as i32),
            })
            .collect(),
        pill_left: round(pill_left),
        pill_right: round(pill_right),
    }
}

/// Which slot or gap `x` is in.
fn anchor(rest_left: &[f64], x: f64, pitch: f64) -> Anchor {
    let index = rest_left.iter().rposition(|&a| a <= x).unwrap_or(0);
    let into = x - rest_left[index];
    if into < pitch || index + 1 == rest_left.len() {
        Anchor::Slot {
            index,
            fraction: (into / pitch).clamp(0.0, 1.0),
        }
    } else {
        Anchor::GapAfter { index }
    }
}

/// Every magnified slot's left edge: the anchor placed under the pointer, then chained right
/// and left keeping each rest gap beyond `pitch`.
fn chain(rest_left: &[f64], widths: &[f64], anchor: Anchor, x: f64, pitch: f64) -> Vec<f64> {
    let n = rest_left.len();
    let extra_after = |j: usize| rest_left[j + 1] - rest_left[j] - pitch;
    let mut left = vec![0.0; n];
    let start = match anchor {
        Anchor::Slot { index, fraction } => {
            left[index] = x - fraction * widths[index];
            index
        }
        Anchor::GapAfter { index } => {
            left[index + 1] = rest_left[index + 1];
            index + 1
        }
    };
    for j in start + 1..n {
        left[j] = left[j - 1] + widths[j - 1] + extra_after(j - 1);
    }
    for j in (0..start).rev() {
        left[j] = left[j + 1] - widths[j] - extra_after(j);
    }
    left
}

/// How far to move everything so the pill keeps its distance from both output edges.
fn clamp_shift(pill_left: f64, pill_right: f64, p: &MagnifyParams) -> f64 {
    let edge = f64::from(p.edge_clamp.0);
    let right_limit = f64::from(p.output_width.0) - edge;
    if pill_left < edge {
        edge - pill_left
    } else if pill_right > right_limit {
        right_limit - pill_right
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{Layout, magnify, rest_centers};
    use crate::magnify::MagnifyParams;
    use sill_model::units::{Cpx, Permille};

    /// An output wide enough that no clamp is active for up to 30 tiles at full zoom.
    fn wide() -> MagnifyParams {
        MagnifyParams::design_defaults(Cpx::px(6000))
    }

    fn slot_edges(layout: &Layout, gap: i32) -> Vec<(i32, i32)> {
        layout
            .tiles
            .iter()
            .map(|t| (t.left.0 - gap / 2, t.left.0 + t.size.0 + gap / 2))
            .collect()
    }

    /// design/10's first spacing (gap 4, pad 8), which the worked numbers below were written
    /// against; the polish pass later moved the defaults to 8 and 6.
    fn first_spacing(p: MagnifyParams) -> MagnifyParams {
        MagnifyParams {
            gap: Cpx::px(4),
            pad: Cpx::px(8),
            ..p
        }
    }

    #[test]
    fn rest_is_centred_and_matches_the_pill_formula() {
        let p = first_spacing(MagnifyParams::design_defaults(Cpx::px(1920)));
        let centers = rest_centers(7, &p);
        // W_0 = 7 * 48 + 6 * 4 + 16 = 376; left = (1920 - 376) / 2 = 772.
        assert_eq!(centers[0], Cpx::px(772 + 8 + 24));
        assert_eq!(centers[6].0 - centers[0].0, Cpx::px(6 * 52).0);
        let rest = magnify(&centers, Cpx::px(0), Permille::ZERO, &p);
        assert_eq!(rest.pill_left, Cpx::px(772));
        assert_eq!(rest.pill_right, Cpx::px(772 + 376));
    }

    #[test]
    fn progress_zero_is_rest_wherever_the_pointer_is() {
        let p = MagnifyParams::design_defaults(Cpx::px(1920));
        let centers = rest_centers(7, &p);
        let rest = magnify(&centers, centers[0], Permille::ZERO, &p);
        for x in (70_000..120_000).step_by(371) {
            assert_eq!(magnify(&centers, Cpx(x), Permille::ZERO, &p), rest, "x {x}");
        }
        for (tile, c) in rest.tiles.iter().zip(&centers) {
            assert_eq!(tile.left.0 + tile.size.0 / 2, c.0);
            assert_eq!(tile.size, Cpx::px(48));
        }
    }

    #[test]
    fn the_tile_under_the_pointer_stays_under_the_pointer() {
        // design/10 §10.8 test 2 (anchor invariant), as a deterministic sweep: 1..=30 tiles,
        // every 37 cpx across the slots, full and half progress.
        let p = wide();
        let gap = p.gap.0;
        for n in 1..=30 {
            let centers = rest_centers(n, &p);
            let rest_slots: Vec<(i32, i32)> = centers
                .iter()
                .map(|c| (c.0 - (p.tile.0 + gap) / 2, c.0 + (p.tile.0 + gap) / 2))
                .collect();
            let (start, end) = (rest_slots[0].0, rest_slots[n - 1].1);
            for progress in [Permille(500), Permille::ONE] {
                for x in (start..end).step_by(37) {
                    let k = rest_slots
                        .iter()
                        .position(|&(a, b)| a <= x && x < b)
                        .unwrap_or(n - 1);
                    let layout = magnify(&centers, Cpx(x), progress, &p);
                    let (a, b) = slot_edges(&layout, gap)[k];
                    assert!(
                        a - 2 <= x && x <= b + 2,
                        "n {n} x {x} P {progress:?}: slot {k} is {a}..{b}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_layout_is_continuous_in_the_pointer() {
        // design/10 §10.8 test 2 (continuity): a 1 cpx pointer move moves nothing more than a
        // few cpx, including across every slot boundary.
        let p = wide();
        for n in [1, 2, 7, 30] {
            let centers = rest_centers(n, &p);
            let start = centers[0].0 - 2600;
            let end = centers[n - 1].0 + 2600;
            let mut before = magnify(&centers, Cpx(start), Permille::ONE, &p);
            for x in start + 1..end {
                let now = magnify(&centers, Cpx(x), Permille::ONE, &p);
                for (i, (a, b)) in before.tiles.iter().zip(&now.tiles).enumerate() {
                    assert!(
                        (a.left.0 - b.left.0).abs() <= 4 && (a.size.0 - b.size.0).abs() <= 4,
                        "n {n} x {x} tile {i}: {a:?} -> {b:?}"
                    );
                }
                before = now;
            }
        }
    }

    #[test]
    fn the_pill_widens_by_the_extra_width_and_the_baseline_order_holds() {
        let p = first_spacing(wide());
        let centers = rest_centers(7, &p);
        let rest = magnify(&centers, centers[3], Permille::ZERO, &p);
        let full = magnify(&centers, centers[3], Permille::ONE, &p);
        let widened =
            (full.pill_right.0 - full.pill_left.0) - (rest.pill_right.0 - rest.pill_left.0);
        assert!((widened - 11_583).abs() <= 10, "{widened}");
        // Pointer on the middle tile's centre: the dock grows symmetrically about it.
        let middle = &full.tiles[3];
        assert!((middle.left.0 + middle.size.0 / 2 - centers[3].0).abs() <= 1);
        for pair in full.tiles.windows(2) {
            assert!(
                pair[0].left.0 + pair[0].size.0 <= pair[1].left.0,
                "{pair:?}"
            );
        }
    }

    #[test]
    fn the_pill_is_clamped_inside_the_output() {
        // Three tiles whose rest pill touches the left clamp (8 px): magnifying the first tile
        // would push the pill past the edge, so everything shifts right by exactly the excess.
        let p = MagnifyParams::design_defaults(Cpx::px(1000));
        let first = 8 + 8 + 24;
        let centers: Vec<Cpx> = (0..3).map(|k| Cpx::px(first + 52 * k)).collect();
        let left = magnify(&centers, centers[0], Permille::ONE, &p);
        assert_eq!(left.pill_left, Cpx::px(8), "{left:?}");
        // The mirror at the right edge.
        let last = 1000 - 8 - 8 - 24;
        let centers: Vec<Cpx> = (0..3).map(|k| Cpx::px(last - 52 * (2 - k))).collect();
        let right = magnify(&centers, centers[2], Permille::ONE, &p);
        assert_eq!(right.pill_right, Cpx::px(992), "{right:?}");
    }

    #[test]
    fn a_separator_gap_keeps_its_width_and_follows_the_pointer() {
        let p = wide();
        let mut centers = rest_centers(4, &p);
        // Push the last tile 13 px further right: a separator block (1 + 2 x 6).
        centers[3] = Cpx(centers[3].0 + 1300);
        let x_in_gap = centers[2].0 + 2600 + 600;
        let layout = magnify(&centers, Cpx(x_in_gap), Permille::ONE, &p);
        let slots = slot_edges(&layout, p.gap.0);
        assert!(
            slots[2].1 <= x_in_gap && x_in_gap <= slots[3].0,
            "{slots:?}"
        );
        assert!((slots[3].0 - slots[2].1 - 1300).abs() <= 2, "{slots:?}");
        // Continuity entering and leaving the gap.
        for x in [
            centers[2].0 + 2599,
            centers[2].0 + 2600,
            centers[3].0 - 2601,
            centers[3].0 - 2600,
        ] {
            let a = magnify(&centers, Cpx(x), Permille::ONE, &p);
            let b = magnify(&centers, Cpx(x + 1), Permille::ONE, &p);
            for (s, t) in a.tiles.iter().zip(&b.tiles) {
                assert!((s.left.0 - t.left.0).abs() <= 4, "x {x}: {s:?} {t:?}");
            }
        }
    }

    #[test]
    fn no_tiles_is_an_empty_pill_in_the_middle() {
        let p = MagnifyParams::design_defaults(Cpx::px(1000));
        let layout = magnify(&[], Cpx(0), Permille::ONE, &p);
        assert!(layout.tiles.is_empty());
        assert_eq!(layout.pill_left, Cpx::px(500));
        assert_eq!(layout.pill_right, Cpx::px(500));
    }
}
