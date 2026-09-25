//! How the column draws its deck, pure: the cards' order for the corner they stack in, and each
//! card's pose as the stylesheet's attributes. The motion itself is the stylesheet's transitions
//! on quire's duration and easing tokens (design/13 §13.3.6: in `--t-big --e-spring`, spring
//! back `--t-move --e-spring`, out `--t-move --e-exit`), until quire has a banner stack host and
//! swipe motion of its own (FINDINGS Q121, Q122).

use sill_settings::{BannerEntry, BannerPosition};

use super::deck::{Entry, Pose};
use super::machine::Banner;

/// Which transition a card's transform takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transit {
    /// Its entrance: `--t-big --e-spring`.
    Enter,
    /// Back to rest after a drag, or a hover: `--t-move --e-spring`.
    Spring,
    /// Following the pointer: none.
    Live,
    /// Its exit: `--t-move --e-exit`.
    Exit,
}

impl Transit {
    pub fn slug(self) -> &'static str {
        match self {
            Transit::Enter => "enter",
            Transit::Spring => "spring",
            Transit::Live => "live",
            Transit::Exit => "exit",
        }
    }
}

/// One card's pose as drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardLook {
    /// `data-pose`: `offstage`, `rest` or `follow`.
    pub pose: &'static str,
    /// The inline transform while following a drag.
    pub shift: Option<String>,
    pub transit: Transit,
}

/// `entry`'s look.
pub fn card_look(entry: &Entry) -> CardLook {
    let transit = match entry.banner {
        Banner::Entering { .. } => Transit::Enter,
        Banner::Dragging { .. } => Transit::Live,
        Banner::Leaving { .. } | Banner::Gone => Transit::Exit,
        Banner::Shown { .. } | Banner::Hovered { .. } => Transit::Spring,
    };
    let (pose, shift) = match entry.pose {
        Pose::Offstage => ("offstage", None),
        Pose::Rest => ("rest", None),
        Pose::Follow(dx) => (
            "follow",
            Some(format!("transform:translateX({}px)", f64::from(dx.0) / 100.0)),
        ),
    };
    CardLook {
        pose,
        shift,
        transit,
    }
}

/// `drawn` (newest first) top down for `corner`: newest on top at the top right, at the bottom
/// at the bottom right.
pub fn in_corner<T>(drawn: Vec<T>, corner: BannerPosition) -> Vec<T> {
    match corner {
        BannerPosition::TopRight => drawn,
        BannerPosition::BottomRight => drawn.into_iter().rev().collect(),
    }
}

/// `data-corner` on the column.
pub fn corner_slug(corner: BannerPosition) -> &'static str {
    match corner {
        BannerPosition::TopRight => "top",
        BannerPosition::BottomRight => "bottom",
    }
}

/// `data-from` on a card: where its entrance starts.
pub fn entry_slug(entry: BannerEntry) -> &'static str {
    match entry {
        BannerEntry::FromRight => "right",
        BannerEntry::FromBelow => "below",
    }
}

#[cfg(test)]
mod tests {
    use super::{CardLook, Transit, card_look, in_corner};
    use crate::surfaces::banners::deck::{Entry, EntryKey, Pointer, Pose};
    use crate::surfaces::banners::machine::{Banner, Deadline, Leave, Remaining};
    use crate::surfaces::banners::stack::Card;
    use crate::surfaces::notification_card::Expansion;
    use sill_services::{AppKey, BannerLife, Cpx, NotificationId, Stamp};
    use sill_settings::BannerPosition;

    fn entry(banner: Banner, pose: Pose) -> Entry {
        Entry {
            key: EntryKey(0),
            card: Card {
                app: AppKey("a".to_owned()),
                grouped: vec![NotificationId(1)],
                life: BannerLife::Expires,
            },
            banner,
            pointer: Pointer::Away,
            pose,
            expansion: Expansion::Collapsed,
        }
    }

    #[test]
    fn each_phase_takes_its_transition() {
        let look = |pose: &'static str, transit| CardLook {
            pose,
            shift: None,
            transit,
        };
        let cases = [
            (
                entry(Banner::Entering { since: Stamp(0) }, Pose::Offstage),
                look("offstage", Transit::Enter),
            ),
            (
                entry(
                    Banner::Shown {
                        deadline: Deadline::Never,
                    },
                    Pose::Rest,
                ),
                look("rest", Transit::Spring),
            ),
            (
                entry(
                    Banner::Dragging {
                        dx: Cpx::px(30),
                        remaining: Remaining::Forever,
                    },
                    Pose::Follow(Cpx(3050)),
                ),
                CardLook {
                    pose: "follow",
                    shift: Some("transform:translateX(30.5px)".to_owned()),
                    transit: Transit::Live,
                },
            ),
            (
                entry(
                    Banner::Leaving {
                        since: Stamp(0),
                        why: Leave::Dismissed,
                    },
                    Pose::Offstage,
                ),
                look("offstage", Transit::Exit),
            ),
        ];
        for (e, want) in cases {
            assert_eq!(card_look(&e), want, "{:?}", e.banner);
        }
    }

    #[test]
    fn the_bottom_corner_puts_the_newest_last() {
        assert_eq!(in_corner(vec![3, 2, 1], BannerPosition::TopRight), [3, 2, 1]);
        assert_eq!(in_corner(vec![3, 2, 1], BannerPosition::BottomRight), [1, 2, 3]);
    }
}
