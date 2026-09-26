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
use sill_surfaces::surfaces::launcher::script::{LauncherScript, Scripted, ScriptedInput};
use sill_surfaces::surfaces::launcher::{LauncherKey, LauncherLink, LauncherProviders};
use tokio::sync::watch;

/// What the harness's root provides.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    link: LauncherLink,
    providers: LauncherProviders,
    script: LauncherScript,
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
    use_context_provider(|| world.script.clone());
    rsx! {
        Launcher { output: fake::output() }
    }
}

/// An open launcher and what it talks to.
pub struct Open {
    pub harness: Harness,
    pub feeds: LauncherFeeds,
    pub link: LauncherLink,
    /// The keys the harness cannot press (Ctrl+K), as the demo scripts them.
    pub script: watch::Sender<Scripted>,
}

impl Open {
    /// Press `key` through the launcher's script, as `examples/launcher_demo.rs` does.
    pub fn scripted(&mut self, key: LauncherKey) {
        self.script.send_modify(|s| {
            s.count += 1;
            s.input = Some(ScriptedInput::Key(key));
        });
        self.harness.advance(Duration::from_millis(20));
    }
}

/// The launcher opened with `query` typed, once its rows (or its empty state) have drawn.
pub fn open(query: &str) -> Open {
    let (services, feeds) = launcher_support::services();
    let link = LauncherLink::new();
    link.open(fake::output().id, query.to_owned());
    let (script, script_rx) = watch::channel(Scripted::default());
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(ShellFile::default()),
            link: link.clone(),
            providers: launcher_support::providers(),
            script: LauncherScript(script_rx),
        });
    });
    let viewport = Viewport {
        width: 680 + 360 + 84,
        height: 460 + 66,
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
        script,
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
