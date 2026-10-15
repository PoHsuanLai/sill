//! The apps a fresh account starts with, and the pure decisions that put them in the dock and
//! the launcher's history.

use sill_launcher::{AppId, Frecency, ItemId, ItemKey, ProviderKind, UnixTime, Use};

use crate::apps::{App, AppIndex, Listing};
use crate::pins::DockPins;

/// Whether the file a seed would create is there already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Store {
    /// Never written: this is a fresh account.
    Missing,
    /// Written once, whatever it holds now.
    Present,
}

/// One place in the Mac's order.
enum Wanted {
    /// The desktop's own app with this desktop id.
    Id(&'static str),
    /// Whichever terminal is installed.
    Terminal,
}

/// The Mac's order: Mail, then the viewer, then the terminal, with Settings last (System
/// Settings sits at the end of the Mac's own dock).
const ORDER: &[Wanted] = &[
    Wanted::Id("mailo"),
    Wanted::Id("org.quire.Anyview"),
    Wanted::Terminal,
    Wanted::Id("org.quire.Settings"),
];

/// The category that names a terminal in a desktop entry.
const TERMINAL: &str = "TerminalEmulator";

/// The installed apps of [`ORDER`], in that order: a listed entry with that id, the first
/// listed terminal by id for the terminal's place. A place nothing fills is skipped.
pub fn first_apps(installed: &AppIndex) -> Vec<AppId> {
    ORDER
        .iter()
        .filter_map(|wanted| match wanted {
            Wanted::Id(id) => installed
                .get(&AppId((*id).to_owned()))
                .filter(|app| listed(app)),
            Wanted::Terminal => installed
                .apps
                .iter()
                .find(|app| listed(app) && app.categories.iter().any(|c| c == TERMINAL)),
        })
        .map(|app| app.id.clone())
        .collect()
}

fn listed(app: &App) -> bool {
    app.listing == Listing::Shown
}

/// The pins a fresh account's dock starts with: [`first_apps`] when the dock's file was never
/// written and at least one of them is installed. `None` leaves the dock alone.
pub fn first_dock(store: Store, installed: &AppIndex) -> Option<DockPins> {
    match store {
        Store::Present => None,
        Store::Missing => {
            let pinned = first_apps(installed);
            (!pinned.is_empty()).then_some(DockPins { pinned })
        }
    }
}

/// The history a fresh account's launcher starts with, so its Recent row is not empty:
/// [`first_apps`] as if used at `now`, the first most (an empty query ranks by frecency alone,
/// and a larger count ranks higher). `None` leaves the history alone.
pub fn first_frecency(store: Store, installed: &AppIndex, now: UnixTime) -> Option<Frecency> {
    match store {
        Store::Present => None,
        Store::Missing => {
            let apps = first_apps(installed);
            let total = u32::try_from(apps.len()).unwrap_or(u32::MAX);
            let entries = apps
                .iter()
                .zip((1..=total).rev())
                .map(|(app, count)| {
                    (
                        ItemKey::of(ProviderKind::Apps, &ItemId(app.0.clone())),
                        Use { count, last: now },
                    )
                })
                .collect();
            (!apps.is_empty()).then_some(Frecency { entries })
        }
    }
}
