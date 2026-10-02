//! The bar's menu tracker: quire's `MenuTrack` (design/13 §13.4), keyed by [`MenuId`], with the
//! bar's `Stamp` time and integer points converted at this boundary.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ds::base::time::stamp::Stamp;
use ds::prelude::*;
use ds::stack::menu_track::types::{
    MenuTarget, MenuTiming, MenuTrack, MenuTrackEffect, MenuTrackEvent,
};
use sill_settings::MenusSettings;

/// A bar menu: one per title and status item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MenuId(pub u32);

/// The bar's tracker.
pub type Track = MenuTrack<MenuId>;
/// One input to it.
pub type Event = MenuTrackEvent<MenuId>;
/// What is under the pointer.
pub type Target = MenuTarget<MenuId>;
/// One thing the bar must do.
pub type Effect = MenuTrackEffect<MenuId>;

/// `menus.submenu_delay_ms` and `menus.submenu_triangle_timeout_ms` as the tracker's timings.
pub fn timing(menus: &MenusSettings) -> MenuTiming {
    MenuTiming {
        submenu_delay: Duration::from_millis(u64::from(menus.submenu_delay_ms.0)),
        triangle_timeout: Duration::from_millis(u64::from(menus.submenu_triangle_timeout_ms.0)),
    }
}

/// A pointer position in whole logical pixels, as the tracker's point.
pub fn point(at: shell_host::Point) -> Point {
    Point {
        x: Px(at.x as f32),
        y: Px(at.y as f32),
    }
}

/// `track` after `event` at `now`, and what the bar must do.
pub fn step(track: Track, event: Event, now: Stamp) -> (Track, Vec<Effect>) {
    track.step(event, instant(now))
}

/// `stamp` on the tracker's clock: milliseconds after a fixed epoch.
fn instant(stamp: Stamp) -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now) + Duration::from_millis(stamp.0)
}
