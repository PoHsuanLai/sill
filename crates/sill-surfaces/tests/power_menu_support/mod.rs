//! The real `PowerMenu` in a `ds_native::Harness` (a Blitz document, no window) on the fake
//! services and a dry-run session backend, following a `PowerMenuLink` the test writes, as the
//! daemon does. Nothing here can reach logind: the backend is always a `DryRunSession`.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../support/fake.rs"]
pub mod fake;

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_model::session::Session;
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::PowerMenu;
use sill_surfaces::hooks::PinnedAppearance;
use sill_surfaces::services::Services;
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
    // Not the desktop's portal and appearance file: they answer off-thread.
    use_context_provider(PinnedAppearance::default);
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
    let harness =
        Harness::with_config(app, HarnessConfig::new(VIEWPORT).with_clock(Clock::Virtual));
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

/// Advance until `done` holds (`settle_until`, which fails with the document if it never does);
/// then one more frame's worth.
pub fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    ds_native::harness::settle_until(harness, done);
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

/// The sheet's phase: its panel's `data-look` (`open` or `leaving`), `closed` with no sheet.
pub fn look(harness: &Harness) -> Option<String> {
    match harness.count(".sill-power-menu-panel") {
        0 => Some("closed".to_owned()),
        _ => harness.attr(".sill-power-menu-panel", "data-look"),
    }
}

/// Whether the menu is showing.
pub fn is_open(link: &PowerMenuLink) -> bool {
    link.now().opening().is_some()
}

/// The `n`th button (1-based, `SessionAction::POWER_MENU` order).
pub fn nth_button(n: usize) -> String {
    format!(".sill-power-menu-buttons .ds-button:nth-child({n})")
}
