//! The real `SwitcherPanel` in a `ds_native::Harness` (a Blitz document, no window) on the fake
//! services, following a `SwitcherLink` the test writes, as the daemon does.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../support/fake.rs"]
pub mod fake;

use std::cell::RefCell;

use dioxus::prelude::*;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_launcher::WindowKey;
use sill_model::cosmic_wl::{Activity, Sizing, Toplevel, Toplevels, WindowState, WlCommand};
use sill_model::ids::{OutputName, WorkspaceKey};
use sill_services::Services;
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::SwitcherPanel;
use sill_surfaces::surfaces::switcher::{SwitcherAsk, SwitcherLink};

/// What the harness's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    link: SwitcherLink,
}

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

/// The root: the daemon's contexts, then the switcher on the test output.
fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(|| world.settings.clone());
    use_context_provider(|| world.link.clone());
    rsx! {
        SwitcherPanel { output: fake::output() }
    }
}

/// A running switcher and what it talks to.
pub struct Running {
    pub harness: Harness,
    pub feeds: fake::Feeds,
    pub link: SwitcherLink,
}

impl Running {
    /// Ask the switcher on the test output, taken at this instant: the next `advance` runs the
    /// woken switcher at `harness.now()` before it steps any timer.
    pub fn ask(&mut self, ask: SwitcherAsk) {
        self.link.ask(fake::output().id, ask);
    }

    /// The commands sent so far, drained.
    pub fn sent(&mut self) -> Vec<WlCommand> {
        std::iter::from_fn(|| self.feeds.wl.try_recv().ok()).collect()
    }
}

/// Windows of `apps` (app id, window key), most recently activated first.
pub fn windows(apps: &[(&str, u64)]) -> Toplevels {
    let window = |(app_id, key): &(&str, u64)| Toplevel {
        key: WindowKey(*key),
        app_id: (*app_id).to_owned(),
        title: format!("{app_id} {key}"),
        activity: Activity::Inactive,
        window: WindowState::Normal,
        sizing: Sizing::Floating,
        workspaces: vec![WorkspaceKey(2)],
        outputs: vec![OutputName("DP-1".to_owned())],
    };
    Toplevels {
        windows: apps.iter().map(window).collect(),
    }
}

/// The switcher on `file`'s settings over `toplevels`, nothing shown yet.
pub fn start(file: ShellFile, toplevels: Toplevels) -> Running {
    let (services, feeds) = fake::services();
    feeds.cosmic_wl.send_modify(|wl| wl.toplevels = toplevels);
    let link = SwitcherLink::new();
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(file),
            link: link.clone(),
        });
    });
    let viewport = Viewport {
        width: 1280,
        height: 720,
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

/// The tiles' names in order (`aria-label`), and the selected one's.
pub fn tiles(harness: &Harness) -> (Vec<String>, Option<String>) {
    let html = harness.html();
    let mut names = Vec::new();
    let mut selected = None;
    let mut rest = html.as_str();
    while let Some(at) = rest.find("role=\"option\"") {
        rest = &rest[at..];
        let tag = &rest[..rest.find('>').unwrap_or(rest.len())];
        let name = attr(tag, "aria-label").unwrap_or_default();
        if attr(tag, "aria-selected").as_deref() == Some("true") {
            selected = Some(name.clone());
        }
        names.push(name);
        rest = &rest["role=\"option\"".len()..];
    }
    (names, selected)
}

/// A tile's `data-presence` (`leaving` while it folds).
pub fn presence(harness: &Harness, name: &str) -> Option<String> {
    let html = harness.html();
    let needle = format!("aria-label=\"{name}\"");
    let at = html.find(&needle)?;
    let start = html[..at].rfind('<')?;
    let tag = &html[start..at + html[at..].find('>')?];
    attr(tag, "data-presence")
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let rest = &tag[tag.find(&needle)? + needle.len()..];
    Some(rest[..rest.find('"')?].to_owned())
}
