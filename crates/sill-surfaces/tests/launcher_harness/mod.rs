//! The real `Launcher` in a `ds_native::Harness` (a Blitz document, no window), opened on the
//! fake services with their search runner: keys and time as a user gives them. The harness runs
//! on the virtual clock and the runner as a task on its thread, so what a test sees
//! depends only on what it did, never on how loaded the machine is.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../launcher_support/mod.rs"]
pub mod launcher_support;

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds_native::harness::settle_until;
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use launcher_support::{LauncherFeeds, Runner, fake};
use sill_services::Services;
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::Launcher;
use sill_surfaces::hooks::PinnedAppearance;
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
    /// The next harness's search runner, taken by its root.
    static RUNNER: RefCell<Option<Runner>> = const { RefCell::new(None) };
}

/// The root: the daemon's contexts (the harness provides the measurer), then the panel.
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
    use_context_provider(|| world.providers.clone());
    use_context_provider(|| world.script.clone());
    use_hook(|| {
        if let Some(runner) = RUNNER.with(|r| r.borrow_mut().take()) {
            spawn(runner.run());
        }
    });
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
    let (services, feeds, runner) = launcher_support::services_with_runner();
    RUNNER.with(|r| *r.borrow_mut() = Some(runner));
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
    let mut harness = Harness::with_config(app, virtual_clock(viewport));
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

/// `viewport` on the virtual clock.
pub fn virtual_clock(viewport: Viewport) -> HarnessConfig {
    HarnessConfig::new(viewport).with_clock(Clock::Virtual)
}

/// Advance until `done` holds (`settle_until`, which fails with the document if it never does);
/// then one more frame's worth.
pub fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    settle_until(harness, done);
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

/// What a [`watch`] over some frames saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seen {
    Once,
    Never,
}

/// The harness's clock per frame watched, the wall clock let run between frames, and how many
/// frames a watch lasts at most (10 s on the harness's clock, at least 4 s of wall clock).
const FRAME: Duration = Duration::from_millis(5);
const BETWEEN: Duration = Duration::from_millis(2);
const FRAMES: u32 = 2000;

/// Advance frame by frame until `done` holds, letting the wall clock run between frames: what
/// lands off the harness's thread (a PDF page's raster, a text file's head) lands in wall time,
/// never within one `advance`. Watching for a thing that must not happen, `Never`
/// is the pass.
pub fn watch(harness: &mut Harness, done: impl Fn(&Harness) -> bool) -> Seen {
    for _ in 0..FRAMES {
        if done(harness) {
            return Seen::Once;
        }
        std::thread::sleep(BETWEEN);
        harness.advance(FRAME);
    }
    match done(harness) {
        true => Seen::Once,
        false => Seen::Never,
    }
}

/// Wait for work done on another thread (a PDF page's raster, a file's head read) to land,
/// on the wall clock and without moving the harness clock: a loaded machine may take seconds,
/// and advancing the virtual clock meanwhile would cross quire's pending cap and change what is
/// drawn. `advance(ZERO)` runs whatever the landing woke, at the current instant.
pub fn await_off_thread(harness: &mut Harness, done: impl Fn(&Harness) -> bool) -> Seen {
    let deadline = std::time::Instant::now() + OFF_THREAD_LIMIT;
    while std::time::Instant::now() < deadline {
        if done(harness) {
            return Seen::Once;
        }
        std::thread::sleep(Duration::from_millis(10));
        harness.advance(Duration::ZERO);
    }
    match done(harness) {
        true => Seen::Once,
        false => Seen::Never,
    }
}

/// How long [`await_off_thread`] waits on the wall clock.
const OFF_THREAD_LIMIT: Duration = Duration::from_secs(30);

/// The preview pane is up with nothing left loading (quire's `aria-busy` is gone).
pub fn loaded(harness: &Harness) -> bool {
    harness.count(".ds-preview") == 1 && harness.attr(".ds-preview", "aria-busy").is_none()
}
