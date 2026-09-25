//! The real `PowerMenu` in a `ds_native::Harness` (a Blitz document, no window) on the fake
//! services and a dry-run session backend, following a `PowerMenuLink` the test writes, as the
//! daemon does. Nothing here can reach logind: the backend is always a `DryRunSession`.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../support/fake.rs"]
pub mod fake;

use std::cell::RefCell;
use std::time::{Duration, Instant};

use dioxus::prelude::*;
use ds_native::{Harness, Viewport};
use sill_services::{Services, Session};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::PowerMenu;
use sill_surfaces::surfaces::power_menu::{
    DryRunSession, PowerMenuBackend, PowerMenuLink, Preselect,
};

/// What the harness's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    link: PowerMenuLink,
    backend: PowerMenuBackend,
}

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

/// The root: the daemon's contexts and the dry-run backend, then the menu on the test output.
fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(|| world.settings.clone());
    use_context_provider(|| world.link.clone());
    use_context_provider(|| world.backend.clone());
    rsx! {
        PowerMenu { output: fake::output() }
    }
}

/// The viewport's size, as the rects report it.
pub const WIDTH: f32 = 1280.0;
pub const HEIGHT: f32 = 800.0;

/// The viewport: a small output.
pub const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

/// A running menu and what it talks to.
pub struct Running {
    pub harness: Harness,
    pub feeds: fake::Feeds,
    pub link: PowerMenuLink,
    /// What the menu has "run".
    pub session: DryRunSession,
}

/// The menu on `file`'s settings with `offers`, hidden.
pub fn start(file: ShellFile, offers: Session) -> Running {
    let (services, feeds) = fake::services();
    let link = PowerMenuLink::new();
    let session = DryRunSession::new(offers, None);
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file),
            link: link.clone(),
            backend: PowerMenuBackend::dry_run(session.clone()),
        });
    });
    let harness = Harness::new(app, VIEWPORT);
    Running {
        harness,
        feeds,
        link,
        session,
    }
}

/// The menu opened with `preselect`, once its sheet has settled.
pub fn opened(file: ShellFile, offers: Session, preselect: Preselect) -> Running {
    let mut running = start(file, offers);
    running.harness.advance(Duration::from_millis(40));
    running.link.open(fake::output().id, preselect);
    settle(&mut running.harness, |h| {
        h.count(".ds-sheet") == 1 && ring_on_primary(h)
    });
    running.harness.advance(Duration::from_millis(600));
    running
}

/// Advance until `done` holds, at most two seconds; then one more frame's worth.
pub fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    let until = Instant::now() + Duration::from_secs(2);
    while !done(harness) && Instant::now() < until {
        harness.advance(Duration::from_millis(20));
    }
    harness.advance(Duration::from_millis(20));
}

/// The focused button's label: the one drawn `Primary`.
pub fn primary(harness: &Harness) -> Option<String> {
    harness.text_of(".ds-button[*|data-variant=primary]")
}

/// Whether the keyboard focus is on the `Primary` button (quire's focus ring is there).
pub fn ring_on_primary(harness: &Harness) -> bool {
    harness.is_focused(".ds-button[*|data-variant=primary]")
}

/// The room's `data-look`: `closed`, `open` or `leaving`.
pub fn look(harness: &Harness) -> Option<String> {
    harness.attr(".sill-power-menu", "data-look")
}

/// Whether the menu is showing.
pub fn is_open(link: &PowerMenuLink) -> bool {
    link.now().opening().is_some()
}

/// The `n`th button (1-based, `SessionAction::POWER_MENU` order).
pub fn nth_button(n: usize) -> String {
    format!(".sill-power-menu-buttons .ds-button:nth-child({n})")
}
