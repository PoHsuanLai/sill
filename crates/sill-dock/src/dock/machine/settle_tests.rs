//! The dock holds a `FrameDrive` from `RequestFrames` to `StopFrames`, so
//! every motion's settle must be reachable at any frame rate. Nested cosmic-comp calls back
//! every 1-2 ms, KWin every 16.7; both are run here, frame by frame, until the machine stops
//! frames.

use super::params::tests::params;
use super::{AttentionKind, step};
use super::{Button, DockEffect, DockEvent, DockState, DropZone, Hit, Point, TileApp, TileId};
use ds::base::time::stamp::Stamp;
use sill_model::units::Cpx;

/// The tick rates: 1 ms, and 16.7 ms as alternating 17 and 16 ms frames.
const RATES: &[(&str, &[u64])] = &[("1 ms", &[1]), ("16.7 ms", &[17, 17, 16])];

fn over(x: i32) -> DockEvent {
    DockEvent::PointerMove {
        at: Point {
            x: Cpx::px(x),
            y: Cpx::px(120),
        },
        hit: Hit::Tile {
            tile: TileId(1),
            app: TileApp::Inactive,
        },
        zone: DropZone::Slot(0),
    }
}

/// Feed frames every `ticks` ms from `dock` at `from` until the machine stops frames; the
/// time it took, or `None` within `limit_ms`.
fn settle(
    mut dock: DockState,
    from: u64,
    ticks: &[u64],
    limit_ms: u64,
) -> Option<(u64, DockState)> {
    let p = params();
    let mut now = from;
    for tick in ticks.iter().cycle() {
        now += tick;
        if now - from > limit_ms {
            return None;
        }
        let (next, out) = step(dock, DockEvent::Frame, Stamp(now), &p);
        dock = next;
        if out.contains(&DockEffect::StopFrames) {
            return Some((now - from, dock));
        }
    }
    None
}

#[test]
fn hover_and_leave_settle_at_any_tick_rate() {
    let p = params();
    for (rate, ticks) in RATES {
        // Enter: grows over 120 ms, then one frame draws the last move and the next stops.
        let (entered, _) = step(DockState::default(), over(900), Stamp(0), &p);
        let (took, tracking) = settle(entered, 0, ticks, 500).unwrap_or_else(|| {
            panic!("{rate}: the enter never stopped frames");
        });
        assert!(
            (120..=160).contains(&took),
            "{rate}: enter settled after {took} ms"
        );
        // A move while tracking: one frame, then stop.
        let (moved, _) = step(tracking, over(930), Stamp(1000), &p);
        let (took, tracking) = settle(moved, 1000, ticks, 100).unwrap_or_else(|| {
            panic!("{rate}: a tracked move never stopped frames");
        });
        assert!(took <= 2 * 17, "{rate}: a move settled after {took} ms");
        // Leave: shrinks over 200 ms and stops at rest.
        let (left, _) = step(tracking, DockEvent::PointerLeave, Stamp(2000), &p);
        let (took, rest) = settle(left, 2000, ticks, 500).unwrap_or_else(|| {
            panic!("{rate}: the leave never stopped frames");
        });
        assert!(
            (200..=220).contains(&took),
            "{rate}: leave settled after {took} ms"
        );
        assert_eq!(rest.hover, super::Hover::Rest, "{rate}");
    }
}

#[test]
fn bounces_and_a_released_drag_settle_at_any_tick_rate() {
    let p = params();
    for (rate, ticks) in RATES {
        let (bouncing, _) = step(
            DockState::default(),
            DockEvent::Attention {
                tile: TileId(1),
                kind: AttentionKind::Informational,
            },
            Stamp(0),
            &p,
        );
        let (took, _) = settle(bouncing, 0, ticks, 1500).unwrap_or_else(|| {
            panic!("{rate}: an informational bounce never stopped frames");
        });
        assert!(
            (1000..=1020).contains(&took),
            "{rate}: bounce settled after {took} ms"
        );
        // Press, drag 20 px, release: the drag ends, the hover resumes and settles.
        let press = DockEvent::Press {
            button: Button::Primary,
            at: Point {
                x: Cpx::px(900),
                y: Cpx::px(120),
            },
            hit: Hit::Tile {
                tile: TileId(1),
                app: TileApp::Inactive,
            },
        };
        let dock = [(0, over(900)), (200, press), (210, over(920))]
            .into_iter()
            .fold(DockState::default(), |d, (ms, e)| {
                step(d, e, Stamp(ms), &p).0
            });
        let (released, _) = step(
            dock,
            DockEvent::Release {
                button: Button::Primary,
                at: Point {
                    x: Cpx::px(920),
                    y: Cpx::px(120),
                },
                hit: Hit::Pill,
                modifier: super::Modifier::None,
            },
            Stamp(300),
            &p,
        );
        assert!(
            settle(released, 300, ticks, 500).is_some(),
            "{rate}: the drop never settled"
        );
    }
}
