//! Bounces (design/10 §10.3.5, §10.5.3): launch, informational and critical, ballistic
//! `lift(u) = A 4u(1 - u)`, and a stop that always finishes the current period.

use ds::base::time::stamp::{Ms, Stamp};
use sill_model::units::Cpx;
use sill_settings::Bounce as BounceSetting;

use super::params::DockParams;
use super::types::{AppChange, AttentionKind, DockEffect, DockEvent, DockState, TileId};

/// Why a tile bounces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BounceKind {
    /// Until the app maps a window, its scope exits, or `deadline`.
    Launch { deadline: Stamp },
    /// Two periods (1 s).
    Informational,
    /// Until the app is activated or its urgency clears.
    Critical,
}

/// One tile's bounce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bounce {
    Bouncing {
        kind: BounceKind,
        since: Stamp,
    },
    /// Stopping: lands at `until`, the end of the period it was in (never mid-air).
    Finishing {
        since: Stamp,
        until: Stamp,
    },
}

/// `tile` starts bouncing (a tile already bouncing keeps its phase: no restart mid-air).
pub(crate) fn start(
    mut dock: DockState,
    tile: TileId,
    kind: BounceKind,
    now: Stamp,
    out: &mut Vec<DockEffect>,
) -> DockState {
    if !dock.bounces.iter().any(|(t, _)| *t == tile) {
        dock.bounces
            .push((tile, Bounce::Bouncing { kind, since: now }));
        out.push(DockEffect::RequestFrames);
    }
    dock
}

/// The height of a tile's bounce at `now`.
pub fn lift(bounce: &Bounce, now: Stamp, p: &DockParams) -> Cpx {
    let since = match *bounce {
        Bounce::Bouncing { since, .. } | Bounce::Finishing { since, .. } => since,
    };
    let period = p.bounce_period_ms.max(1);
    let u = (now.since(since) % period) as f64 / period as f64;
    Cpx((f64::from(p.amplitude.0) * 4.0 * u * (1.0 - u)).round() as i32)
}

/// The bounce half of [`super::step`].
pub(crate) fn step(
    dock: DockState,
    event: &DockEvent,
    now: Stamp,
    p: &DockParams,
    out: &mut Vec<DockEffect>,
) -> DockState {
    match *event {
        DockEvent::Attention { tile, kind } if p.bounce == BounceSetting::On => {
            let kind = match kind {
                AttentionKind::Informational => BounceKind::Informational,
                AttentionKind::Critical => BounceKind::Critical,
            };
            start(dock, tile, kind, now, out)
        }
        DockEvent::AppState { tile, change } => stop_for(dock, tile, change, now, p),
        DockEvent::Frame => expire(dock, now, p),
        _ => dock,
    }
}

/// Stop the bounces `change` ends for `tile`.
fn stop_for(
    mut dock: DockState,
    tile: TileId,
    change: AppChange,
    now: Stamp,
    p: &DockParams,
) -> DockState {
    let ends = |kind: BounceKind| {
        matches!(
            (change, kind),
            (
                AppChange::Mapped | AppChange::ScopeExited | AppChange::LaunchFailed,
                BounceKind::Launch { .. }
            ) | (
                AppChange::Activated,
                BounceKind::Launch { .. } | BounceKind::Critical
            ) | (AppChange::UrgencyCleared, BounceKind::Critical)
        )
    };
    for (t, bounce) in &mut dock.bounces {
        if let Bounce::Bouncing { kind, since } = *bounce
            && *t == tile
            && ends(kind)
        {
            *bounce = finish(since, now, p);
        }
    }
    dock
}

/// The end of the period `now` is in.
fn finish(since: Stamp, now: Stamp, p: &DockParams) -> Bounce {
    let period = p.bounce_period_ms.max(1);
    let periods = now.since(since).div_ceil(period);
    Bounce::Finishing {
        since,
        until: since.after(Ms(periods * period)),
    }
}

/// On a frame: land finished bounces, stop informational ones after two periods, finish launch
/// bounces past their deadline.
fn expire(dock: DockState, now: Stamp, p: &DockParams) -> DockState {
    let bounces = dock
        .bounces
        .into_iter()
        .filter_map(|(tile, bounce)| match bounce {
            Bounce::Finishing { until, .. } if now >= until => None,
            Bounce::Bouncing {
                kind: BounceKind::Informational,
                since,
            } if now.since(since) >= 2 * p.bounce_period_ms => None,
            Bounce::Bouncing {
                kind: BounceKind::Launch { deadline },
                since,
            } if now >= deadline => Some((tile, finish(since, now, p))),
            other => Some((tile, other)),
        })
        .collect();
    DockState { bounces, ..dock }
}

#[cfg(test)]
mod tests {
    use super::{Bounce, BounceKind, lift};
    use crate::dock::machine::params::tests::params;
    use crate::dock::machine::types::{AppChange, AttentionKind, DockEvent, DockState, TileId};
    use crate::dock::machine::{DockParams, step};
    use ds::base::time::stamp::{Ms, Stamp};
    use sill_model::units::Cpx;
    use sill_settings::Bounce as BounceSetting;

    const T: TileId = TileId(2);

    fn run(p: &DockParams, events: &[(u64, DockEvent)]) -> DockState {
        events.iter().fold(DockState::default(), |d, &(ms, e)| {
            step(d, e, Stamp(ms), p).0
        })
    }

    fn attention(kind: AttentionKind) -> (u64, DockEvent) {
        (0, DockEvent::Attention { tile: T, kind })
    }

    fn change(ms: u64, change: AppChange) -> (u64, DockEvent) {
        (ms, DockEvent::AppState { tile: T, change })
    }

    fn launch() -> DockState {
        DockState {
            bounces: vec![(
                T,
                Bounce::Bouncing {
                    kind: BounceKind::Launch {
                        deadline: Stamp(10_000),
                    },
                    since: Stamp(0),
                },
            )],
            ..DockState::default()
        }
    }

    fn bouncing(dock: &DockState) -> bool {
        !dock.bounces.is_empty()
    }

    #[test]
    fn lift_is_ballistic_with_its_peak_mid_period() {
        let p = params();
        let b = Bounce::Bouncing {
            kind: BounceKind::Critical,
            since: Stamp(0),
        };
        const CASES: &[(u64, i32)] = &[
            (0, 0),
            (125, 3600),
            (250, 4800),
            (375, 3600),
            (500, 0),
            (750, 4800),
        ];
        for &(ms, want) in CASES {
            assert_eq!(lift(&b, Stamp(ms), &p), Cpx(want), "{ms} ms");
        }
    }

    #[test]
    fn informational_ends_at_one_second() {
        // design/10 §10.8 test 8.
        let p = params();
        assert!(bouncing(&run(
            &p,
            &[
                attention(AttentionKind::Informational),
                (999, DockEvent::Frame)
            ]
        )));
        assert!(!bouncing(&run(
            &p,
            &[
                attention(AttentionKind::Informational),
                (1000, DockEvent::Frame)
            ]
        )));
    }

    #[test]
    fn critical_finishes_the_period_it_is_in_once_activated() {
        let p = params();
        let base = [
            attention(AttentionKind::Critical),
            (5000, DockEvent::Frame),
            change(5100, AppChange::Activated),
        ];
        assert!(bouncing(&run(
            &p,
            &[&base[..], &[(5499, DockEvent::Frame)]].concat()
        )));
        assert!(!bouncing(&run(
            &p,
            &[&base[..], &[(5500, DockEvent::Frame)]].concat()
        )));
        // Urgency clearing ends it the same way; a mapped window does not.
        assert!(!bouncing(&run(
            &p,
            &[
                attention(AttentionKind::Critical),
                change(100, AppChange::UrgencyCleared),
                (500, DockEvent::Frame)
            ]
        )));
        assert!(bouncing(&run(
            &p,
            &[
                attention(AttentionKind::Critical),
                change(100, AppChange::Mapped),
                (5000, DockEvent::Frame)
            ]
        )));
    }

    #[test]
    fn a_launch_mapped_at_1_3_s_stops_at_1_5_s() {
        // design/10 §10.8 test 8: "stops at the end of the period containing 1.3 s (1.5 s)".
        let p = params();
        let after = |events: &[(u64, DockEvent)]| {
            events
                .iter()
                .fold(launch(), |d, &(ms, e)| step(d, e, Stamp(ms), &p).0)
        };
        let mapped = [change(1300, AppChange::Mapped)];
        assert!(bouncing(&after(
            &[&mapped[..], &[(1499, DockEvent::Frame)]].concat()
        )));
        assert!(!bouncing(&after(
            &[&mapped[..], &[(1500, DockEvent::Frame)]].concat()
        )));
        // The cap: still bouncing at 9.9 s with no window, done at 10 s.
        assert!(bouncing(&after(&[(9_900, DockEvent::Frame)])));
        assert!(!bouncing(&after(&[
            (10_000, DockEvent::Frame),
            (10_000, DockEvent::Frame)
        ])));
    }

    #[test]
    fn the_bounce_setting_gates_attention() {
        let off = DockParams {
            bounce: BounceSetting::Off,
            ..params()
        };
        assert!(!bouncing(&run(&off, &[attention(AttentionKind::Critical)])));
    }
}
