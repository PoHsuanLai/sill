//! BannerStack: the notification banners on screen, newest first (design/20 section
//! 1.6, design/13 section 13.3.6). The caller lists the banners it shows; the stack plays each
//! arrival in from past the right edge (`Anim::PanelIn`, `--t-move --e-out`), and each banner
//! the caller stops listing back out past it (`Anim::PanelOut`, `--t-quick --e-exit`), keeping
//! it in the tree until its exit settles; the banners after it then heal into its place, by
//! the height it measured. `on_hidden` hears each banner's key once its exit has settled, so a
//! host whose list is empty then can unmap the surface. The hold timer, the stack's cap and
//! grouping are the caller's (design/30 section 2.10: a banner slides like a toast and a side
//! panel).
//!
//! A card swiped away inside the stack holds where the finger left it and reports at once; the
//! caller's removal makes its row slide out from there (`NotificationCard`'s swipe).

use crate::kept::use_kept;
use crate::notifications::banner_row::BannerRow;
use crate::tokens::notifications::NotificationToken;
use dioxus::prelude::*;
use ds::base::geometry::units::Px;
use ds::base::word::Word;
use ds::motion::presence::Exit;
use ds::motion::roster::RowPitch;
use ds::motion::use_roster::{LeaveBy, RosterSpec, use_roster};
use ds::root::common::Common;

/// A banner's identity: the notification's id, which the server gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BannerKey(pub u32);

/// One banner the caller shows: its key and its card (a `NotificationCard`, usually).
#[derive(Debug, Clone, PartialEq)]
pub struct Banner {
    /// Which notification.
    pub key: BannerKey,
    /// Its card.
    pub card: Element,
}

/// Where the stack stands (`notifications.banner_position`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum BannerPosition {
    /// Under the bar at the right, newest on top; a banner leaving lets the ones below it rise.
    #[default]
    TopRight,
    /// Above the dock at the right, newest at the bottom; a banner leaving lets the ones above
    /// it drop.
    BottomRight,
}

impl BannerPosition {
    /// Which way the rows after a leaving one heal: up into its place at the top right (they
    /// start below it), down at the bottom right (they start above it).
    pub(crate) fn heal_sign(self) -> f32 {
        match self {
            BannerPosition::TopRight => 1.0,
            BannerPosition::BottomRight => -1.0,
        }
    }
}

/// The pitch a banner that measured nothing heals by: the least banner and the gap.
const FALLBACK_PITCH: RowPitch = RowPitch(Px(64.0 + 8.0));

/// The banners, newest first. `gap` overrides `--notifications-stack-gap` (the
/// `notifications.stack_gap_px` a `NotificationMetrics` around the stack writes).
#[component]
pub fn BannerStack(
    banners: Vec<Banner>,
    #[props(default)] position: BannerPosition,
    #[props(default)] gap: Option<Px>,
    #[props(default)] on_hidden: Option<EventHandler<BannerKey>>,
    #[props(default)] common: Common,
) -> Element {
    let keys: Vec<BannerKey> = banners.iter().map(|banner| banner.key).collect();
    let cards = use_kept(
        banners
            .iter()
            .map(|banner| (banner.key, banner.card.clone()))
            .collect(),
    );
    let roster = use_roster(
        keys,
        RosterSpec {
            leave: LeaveBy::Delist,
            exit: Exit::PanelOut,
            pitch: FALLBACK_PITCH,
            on_settled: Some(EventHandler::new(move |key| {
                cards.forget(&key);
                if let Some(on_hidden) = on_hidden {
                    on_hidden.call(key);
                }
            })),
        },
    );
    let pitches = roster.pitches();
    let style = gap.map(|gap| NotificationToken::StackGap.write(&format!("{}px", gap.0)));
    let class = common.class("ds-banner-stack");
    let data = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "data-position": position.slug(),
            role: "log",
            "aria-label": "Notifications",
            style,
            onmounted: move |event| common.mounted(event),
            ..data,
            for entry in roster.entries() {
                BannerRow {
                    key: "{entry.key.0}",
                    banner: entry.key,
                    presence: entry.presence,
                    heal: entry.heal,
                    position,
                    pitches,
                    card: cards.of(&entry.key).unwrap_or_else(|| rsx! {}),
                }
            }
        }
    }
}
