//! One banner's life (design/13 §13.4 `Banner`, §13.5): hold, hover pause and resume with at
//! least `hover_min_remaining`, swipe right to dismiss, act, close.
//!
//! design/13 §13.3.6 says a 5000 ms lifetime; design/22's `notifications.banner_hold_ms` and
//! design/20 §1.6 (`ToastHold`) say 5200. The key wins (FINDINGS.md).

use sill_services::{Cpx, Stamp};
use sill_settings::NotificationsSettings;

/// When a shown banner leaves by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deadline {
    At(Stamp),
    /// An alert: urgency critical, `resident`, or `expire_timeout == 0`.
    Never,
}

/// How much life a paused banner has left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remaining {
    Ms(u64),
    Forever,
}

/// Why a banner leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leave {
    Timeout,
    Dismissed,
    Acted,
}

/// Whether a banner expires (a banner) or persists (an alert).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Persistence {
    Expires,
    Persists,
}

/// The banner's phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Banner {
    Entering {
        since: Stamp,
    },
    Shown {
        deadline: Deadline,
    },
    Hovered {
        remaining: Remaining,
    },
    /// Being dragged `dx` (raw, before damping); `remaining` resumes a spring-back.
    Dragging {
        dx: Cpx,
        remaining: Remaining,
    },
    Leaving {
        since: Stamp,
        why: Leave,
    },
    Gone,
}

/// One input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerIn {
    /// The entrance animation finished.
    Entered,
    Tick,
    HoverIn,
    HoverOut,
    DragStart,
    DragMove {
        dx: Cpx,
    },
    /// Release with the pointer's horizontal velocity, px/s.
    DragEnd {
        velocity_px_s: i32,
    },
    /// A horizontal scroll gesture over the banner ended, summed, with its release velocity.
    Swipe {
        dx: Cpx,
        velocity_px_s: i32,
    },
    /// The body was clicked (`ActionInvoked("default")`).
    Clicked,
    /// An action button was clicked.
    Action,
    /// The close button, `CloseNotification`, or a replacement.
    Close,
    /// The exit animation finished.
    Left,
}

/// One thing the surface must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerOut {
    RequestTick(Stamp),
    /// Expand (body to 6 lines, actions, close button).
    Expand,
    Collapse,
    /// Draw the banner at this x offset (damped left of zero).
    Offset(Cpx),
    SpringBack,
    /// Start the exit and tell the client why (`NotificationClosed` reason: 1 expired, 2
    /// dismissed; an action is `ActionInvoked` first).
    Leave(Leave),
    /// Remove the banner; the stack heals.
    Remove,
}

/// The banner's numbers, from `notifications.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BannerParams {
    pub hold_ms: u64,
    pub hover_min_remaining_ms: u64,
    pub swipe_dismiss: Cpx,
    pub swipe_velocity_px_s: i32,
    /// Leftward damping, thousandths.
    pub damping: u32,
}

impl BannerParams {
    pub fn new(n: &NotificationsSettings) -> BannerParams {
        BannerParams {
            hold_ms: u64::from(n.banner_hold_ms.0),
            hover_min_remaining_ms: u64::from(n.hover_min_remaining_ms.0),
            swipe_dismiss: Cpx::px(i32::from(n.swipe_dismiss_px.0)),
            swipe_velocity_px_s: n.swipe_dismiss_velocity_px_s.0.round() as i32,
            damping: u32::from(n.swipe_damping.0),
        }
    }
}

/// `b` after `input` at `now`.
pub fn step(
    b: Banner,
    input: BannerIn,
    now: Stamp,
    persistence: Persistence,
    p: &BannerParams,
) -> (Banner, Vec<BannerOut>) {
    let leave = |why| {
        (
            Banner::Leaving { since: now, why },
            vec![BannerOut::Leave(why)],
        )
    };
    let resume = |remaining: Remaining| match remaining {
        Remaining::Forever => (
            Banner::Shown {
                deadline: Deadline::Never,
            },
            None,
        ),
        Remaining::Ms(ms) => {
            let at = now.after(ms.max(p.hover_min_remaining_ms));
            (
                Banner::Shown {
                    deadline: Deadline::At(at),
                },
                Some(BannerOut::RequestTick(at)),
            )
        }
    };
    let remaining_of = |deadline: Deadline| match deadline {
        Deadline::Never => Remaining::Forever,
        Deadline::At(at) => Remaining::Ms(at.since(now)),
    };
    match (b, input) {
        (Banner::Gone, _) => (Banner::Gone, Vec::new()),
        (Banner::Leaving { .. }, BannerIn::Left) => (Banner::Gone, vec![BannerOut::Remove]),
        (Banner::Leaving { .. }, _) => (b, Vec::new()),
        (_, BannerIn::Clicked | BannerIn::Action) => leave(Leave::Acted),
        (_, BannerIn::Close) => leave(Leave::Dismissed),
        (Banner::Entering { .. }, BannerIn::Entered) => match persistence {
            Persistence::Persists => (
                Banner::Shown {
                    deadline: Deadline::Never,
                },
                Vec::new(),
            ),
            Persistence::Expires => {
                let at = now.after(p.hold_ms);
                (
                    Banner::Shown {
                        deadline: Deadline::At(at),
                    },
                    vec![BannerOut::RequestTick(at)],
                )
            }
        },
        (
            Banner::Shown {
                deadline: Deadline::At(at),
            },
            BannerIn::Tick,
        ) if now >= at => leave(Leave::Timeout),
        (Banner::Shown { deadline }, BannerIn::HoverIn) => (
            Banner::Hovered {
                remaining: remaining_of(deadline),
            },
            vec![BannerOut::Expand],
        ),
        (Banner::Hovered { remaining }, BannerIn::HoverOut) => {
            let (shown, tick) = resume(remaining);
            (
                shown,
                [Some(BannerOut::Collapse), tick]
                    .into_iter()
                    .flatten()
                    .collect(),
            )
        }
        (Banner::Shown { deadline }, BannerIn::DragStart) => (
            Banner::Dragging {
                dx: Cpx(0),
                remaining: remaining_of(deadline),
            },
            Vec::new(),
        ),
        (Banner::Hovered { remaining }, BannerIn::DragStart) => (
            Banner::Dragging {
                dx: Cpx(0),
                remaining,
            },
            Vec::new(),
        ),
        (Banner::Dragging { remaining, .. }, BannerIn::DragMove { dx }) => (
            Banner::Dragging { dx, remaining },
            vec![BannerOut::Offset(damped(dx, p))],
        ),
        (Banner::Dragging { dx, remaining }, BannerIn::DragEnd { velocity_px_s }) => {
            if dismisses(dx, velocity_px_s, p) {
                leave(Leave::Dismissed)
            } else {
                let (shown, tick) = resume(remaining);
                (
                    shown,
                    [Some(BannerOut::SpringBack), tick]
                        .into_iter()
                        .flatten()
                        .collect(),
                )
            }
        }
        (Banner::Shown { .. } | Banner::Hovered { .. }, BannerIn::Swipe { dx, velocity_px_s })
            if dismisses(dx, velocity_px_s, p) =>
        {
            leave(Leave::Dismissed)
        }
        (other, _) => (other, Vec::new()),
    }
}

/// design/13 §13.5: `x(dx) = dx` rightward, `0.25 dx` leftward.
pub fn damped(dx: Cpx, p: &BannerParams) -> Cpx {
    if dx.0 >= 0 {
        dx
    } else {
        Cpx(i32::try_from(i64::from(dx.0) * i64::from(p.damping) / 1000).unwrap_or(0))
    }
}

/// design/13 §13.5: dismiss iff `dx >= 80` or `v >= 600 px/s` (rightward).
fn dismisses(dx: Cpx, velocity_px_s: i32, p: &BannerParams) -> bool {
    dx.0 >= p.swipe_dismiss.0 || velocity_px_s >= p.swipe_velocity_px_s
}

#[cfg(test)]
#[path = "banner_tests.rs"]
mod tests;
