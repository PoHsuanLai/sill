//! The real `Osd` in a `ds_native::Harness` (a Blitz document, no window) on the fake
//! services, following an `OsdLink` the test writes, as the daemon does.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../support/fake.rs"]
pub mod fake;

use std::cell::RefCell;

use dioxus::prelude::*;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_services::Services;
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::Osd;
use sill_surfaces::surfaces::osd::{CARD_HEIGHT, CARD_WIDTH, OsdLink, ROOM};

/// What the harness's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    link: OsdLink,
}

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

/// The root: the daemon's contexts, then the indicator on the test output.
fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(|| world.settings.clone());
    use_context_provider(|| world.link.clone());
    rsx! {
        Osd { output: fake::output() }
    }
}

/// A running indicator and what it talks to.
pub struct Running {
    pub harness: Harness,
    pub feeds: fake::Feeds,
    pub link: OsdLink,
}

/// The indicator on `file`'s settings, nothing shown yet.
pub fn start(file: ShellFile) -> Running {
    let (services, feeds) = fake::services();
    let link = OsdLink::new();
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file),
            link: link.clone(),
        });
    });
    let viewport = Viewport {
        width: CARD_WIDTH + 2 * ROOM,
        height: CARD_HEIGHT + 2 * ROOM,
        scale_percent: 100,
    };
    let harness =
        Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual));
    Running {
        harness,
        feeds,
        link,
    }
}

/// The card's presence: quire's `Osd` (`#osd`) is `"hidden"` while `data-shown="hidden"`, else
/// its `data-presence` (`"entering"`, `"present"` or `"leaving"`).
pub fn look(harness: &Harness) -> Option<String> {
    let html = harness.html();
    let rest = &html[html.find("id=\"osd\"")?..];
    let shown = {
        let after = &rest[rest.find("data-shown=\"")? + "data-shown=\"".len()..];
        &after[..after.find('"')?]
    };
    if shown == "hidden" {
        return Some("hidden".to_owned());
    }
    let after = &rest[rest.find("data-presence=\"")? + "data-presence=\"".len()..];
    Some(after[..after.find('"')?].to_owned())
}

/// The level control's `aria-valuenow`.
pub fn level(harness: &Harness) -> Option<u16> {
    let html = harness.html();
    let rest = &html[html.find("class=\"ds-level\"")?..];
    let rest = &rest[rest.find("aria-valuenow=\"")? + "aria-valuenow=\"".len()..];
    rest[..rest.find('"')?].parse().ok()
}

/// The OSD's title (quire's `Osd { label }`, `#osd`'s own `aria-label`).
pub fn label(harness: &Harness) -> Option<String> {
    let html = harness.html();
    let rest = &html[html.find("id=\"osd\"")?..];
    let rest = &rest[rest.find("aria-label=\"")? + "aria-label=\"".len()..];
    Some(rest[..rest.find('"')?].to_owned())
}
