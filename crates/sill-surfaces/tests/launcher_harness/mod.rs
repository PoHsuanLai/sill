//! The real `Launcher` in a `ds_native::Harness` (a Blitz document, no window), opened on the
//! fake services with their search runner: keys and time as a user gives them.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../launcher_support/mod.rs"]
pub mod launcher_support;

use std::cell::RefCell;
use std::time::{Duration, Instant};

use dioxus::prelude::*;
use ds_native::{Harness, Viewport};
use launcher_support::{LauncherFeeds, fake};
use sill_services::Services;
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::Launcher;
use sill_surfaces::surfaces::launcher::{LauncherLink, LauncherProviders};

/// What the harness's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    link: LauncherLink,
    providers: LauncherProviders,
}

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

/// The root: the daemon's contexts (the harness provides the measurer), then the panel.
fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(|| world.settings.clone());
    use_context_provider(|| world.link.clone());
    use_context_provider(|| world.providers.clone());
    rsx! {
        Launcher { output: fake::output() }
    }
}

/// An open launcher and what it talks to.
pub struct Open {
    pub harness: Harness,
    pub feeds: LauncherFeeds,
    pub link: LauncherLink,
}

/// The launcher opened with `query` typed, once its rows (or its empty state) have drawn.
pub fn open(query: &str) -> Open {
    let (services, feeds) = launcher_support::services();
    let link = LauncherLink::new();
    link.open(fake::output().id, query.to_owned());
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(ShellFile::default()),
            link: link.clone(),
            providers: launcher_support::providers(),
        });
    });
    let viewport = Viewport {
        width: 680,
        height: 460,
        scale_percent: 100,
    };
    let mut harness = Harness::new(app, viewport);
    settle(&mut harness, |h| {
        h.count(".ds-menu-item") > 0 || h.count(".ds-menu-empty") > 0
    });
    Open {
        harness,
        feeds,
        link,
    }
}

/// Advance until `done` holds, at most two seconds; then one more frame's worth.
pub fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    let until = Instant::now() + Duration::from_secs(2);
    while !done(harness) && Instant::now() < until {
        harness.advance(Duration::from_millis(20));
    }
    harness.advance(Duration::from_millis(20));
}

/// The rows' titles, in order (marks and all: the text inside each `.ds-menu-title`).
pub fn titles(harness: &Harness) -> Vec<String> {
    let html = harness.html();
    html.split("class=\"ds-menu-title\"")
        .skip(1)
        .filter_map(|rest| {
            let inner = &rest[rest.find('>')? + 1..];
            Some(strip_tags(&inner[..inner.find("</b>")?]))
        })
        .collect()
}

/// The selected row's title.
pub fn selected_title(harness: &Harness) -> Option<String> {
    let html = harness.html();
    let rest = html.split("aria-selected=\"true\"").nth(1)?;
    let rest = &rest[rest.find("class=\"ds-menu-title\"")?..];
    let inner = &rest[rest.find('>')? + 1..];
    Some(strip_tags(&inner[..inner.find("</b>")?]))
}

/// `html` with its tags removed.
fn strip_tags(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    text
}

/// Whether the launcher is open.
pub fn is_open(link: &LauncherLink) -> bool {
    link.now().opening().is_some()
}

/// How many runs a test gets against the race.
const RACE_TRIES: usize = 20;

/// Run `test`, again when it dies of the one known race: quire's `Focus::OnMount` field calls
/// `set_focus` from a task that dioxus-core can poll inside `render_immediate`, while
/// dioxus-native-dom's writer holds the document ("RefCell already borrowed"; FINDINGS Q43,
/// the rect-read race quire fixed, but for focus). Any other panic fails the test at once, and
/// so does the race [`RACE_TRIES`] times over.
pub fn retrying_the_focus_race<T>(test: impl Fn() -> T) -> T {
    for _ in 0..RACE_TRIES {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(&test)) {
            Ok(value) => return value,
            Err(panic) if is_focus_race(&*panic) => continue,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
    panic!("the focus race (Q43) hit {RACE_TRIES} runs in a row");
}

fn is_focus_race(panic: &(dyn std::any::Any + Send)) -> bool {
    let text = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or_default();
    text.contains("already borrowed")
}
