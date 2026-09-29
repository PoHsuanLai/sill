//! One banner's life (design/13 §13.4 `Banner`, §13.5): hold, hover pause and resume with at
//! least `hover_min_remaining`, swiped away, act, close. design/13 places it at
//! `surfaces/notifications/banner.rs`; it sits here, beside its surface.
//!
//! The swipe itself (following the finger, the thresholds, springing back, flying out) is
//! quire's `NotificationCard { swipe }`: the machine hears only that it went ([`BannerIn::Swiped`]).
//!
//! design/13 §13.3.6 says a 5000 ms lifetime; design/22's `notifications.banner_hold_ms` and
//! design/20 §1.6 (`ToastHold`) say 5200. The key wins.

use sill_model::units::Stamp;
use sill_services::BannerLife;
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
    /// The close button, `CloseNotification`, or a replacement.
    Dismissed,
    /// Swiped or dragged away past quire's threshold: kept in the center or dismissed, as
    /// `notifications.swipe` says (the service's decision).
    Swiped,
    Acted,
}

/// Whether a banner expires (a banner) or persists (an alert).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Persistence {
    Expires,
    Persists,
}

impl Persistence {
    /// The service's word for it (`LiveBanner::life`).
    pub fn of(life: BannerLife) -> Persistence {
        match life {
            BannerLife::Expires => Persistence::Expires,
            BannerLife::Persists => Persistence::Persists,
        }
    }
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
    /// Under the pointer (a drag included): the hold is paused with this much left.
    Hovered {
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
    /// The card is on screen: its hold starts.
    Entered,
    Tick,
    HoverIn,
    HoverOut,
    /// quire's card was swiped away (a drag or a horizontal scroll past its threshold).
    Swiped,
    /// The card was pressed (`ActionInvoked("default")`).
    Clicked,
    /// An action button was pressed.
    Action,
    /// The close button, `CloseNotification`, or a replacement.
    Close,
    /// The exit has played (the stack's `on_hidden`).
    Left,
}

/// One thing the surface must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerOut {
    RequestTick(Stamp),
    /// Stop listing the card (its exit plays) and tell the service why
    /// (`commands::leave_commands`).
    Leave(Leave),
    /// Forget the banner; the stack has healed.
    Remove,
}

/// The banner's hold, from `notifications.*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BannerParams {
    pub hold_ms: u64,
    pub hover_min_remaining_ms: u64,
}

impl BannerParams {
    pub fn new(n: &NotificationsSettings) -> BannerParams {
        BannerParams {
            hold_ms: u64::from(n.banner_hold_ms.0),
            hover_min_remaining_ms: u64::from(n.hover_min_remaining_ms.0),
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
    match (b, input) {
        (Banner::Gone, _) => (Banner::Gone, Vec::new()),
        (Banner::Leaving { .. }, BannerIn::Left) => (Banner::Gone, vec![BannerOut::Remove]),
        (Banner::Leaving { .. }, _) => (b, Vec::new()),
        (_, BannerIn::Clicked | BannerIn::Action) => leave(Leave::Acted),
        (_, BannerIn::Close) => leave(Leave::Dismissed),
        (_, BannerIn::Swiped) => leave(Leave::Swiped),
        (Banner::Entering { .. }, BannerIn::Entered) => hold(persistence, now, p),
        (
            Banner::Shown {
                deadline: Deadline::At(at),
            },
            BannerIn::Tick,
        ) if now >= at => leave(Leave::Timeout),
        (Banner::Shown { deadline }, BannerIn::HoverIn) => (
            Banner::Hovered {
                remaining: remaining_of(deadline, now),
            },
            Vec::new(),
        ),
        (Banner::Hovered { remaining }, BannerIn::HoverOut) => resume(remaining, now, p),
        (other, _) => (other, Vec::new()),
    }
}

/// A banner that has just come on screen: held for `hold_ms`, or for ever as an alert.
fn hold(persistence: Persistence, now: Stamp, p: &BannerParams) -> (Banner, Vec<BannerOut>) {
    match persistence {
        Persistence::Persists => (
            Banner::Shown {
                deadline: Deadline::Never,
            },
            Vec::new(),
        ),
        Persistence::Expires => at(now.after(p.hold_ms)),
    }
}

/// A banner the pointer left, with `remaining` (at least `hover_min_remaining_ms`).
fn resume(remaining: Remaining, now: Stamp, p: &BannerParams) -> (Banner, Vec<BannerOut>) {
    match remaining {
        Remaining::Forever => (
            Banner::Shown {
                deadline: Deadline::Never,
            },
            Vec::new(),
        ),
        Remaining::Ms(ms) => at(now.after(ms.max(p.hover_min_remaining_ms))),
    }
}

fn at(deadline: Stamp) -> (Banner, Vec<BannerOut>) {
    (
        Banner::Shown {
            deadline: Deadline::At(deadline),
        },
        vec![BannerOut::RequestTick(deadline)],
    )
}

fn remaining_of(deadline: Deadline, now: Stamp) -> Remaining {
    match deadline {
        Deadline::Never => Remaining::Forever,
        Deadline::At(at) => Remaining::Ms(at.since(now)),
    }
}

#[cfg(test)]
#[path = "machine_tests.rs"]
mod tests;
