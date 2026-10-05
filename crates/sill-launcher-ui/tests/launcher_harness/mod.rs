//! The real `Launcher` surface on `SurfaceHarness`, opened on the
//! fake services with their search runner: keys and time as a user gives them. The harness runs
//! on the virtual clock and the runner as a task on its thread, so what a test sees
//! depends only on what it did, never on how loaded the machine is.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../launcher_support/mod.rs"]
pub mod launcher_support;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::harness::settle_until;
use ds_harness::{Driver, Harness, Input, Query};
use launcher_support::{LauncherFeeds, Runner, fake};
use sill_launcher::LauncherMode;
use sill_launcher_ui::Launcher;
use sill_launcher_ui::launcher::LauncherProviders;
use sill_launcher_ui::launcher::preview::load::{Look, Looking};
use sill_ui_kit::harness::SurfaceHarness;
use sill_ui_kit::links::LauncherLink;

/// What the test gives the root beside the surface's own contexts: the providers and the search
/// runner, which runs as a task on the harness's thread once.
fn in_root(
    providers: LauncherProviders,
    runner: Runner,
    look: Option<Arc<dyn Look>>,
) -> impl Fn() + 'static {
    let runner = Rc::new(RefCell::new(Some(runner)));
    move || {
        use_context_provider(|| providers.clone());
        if let Some(look) = &look {
            use_context_provider(|| Looking(Arc::clone(look)));
        }
        use_hook(|| {
            if let Some(runner) = runner.borrow_mut().take() {
                spawn(runner.run());
            }
        });
    }
}

/// An open launcher and what it talks to.
pub struct Open {
    pub harness: Harness,
    pub feeds: LauncherFeeds,
    pub link: LauncherLink,
}

/// A key a launcher test presses, by what it does to the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    Up,
    Down,
    Enter,
    Escape,
    Space,
    Left,
    /// → with the caret at the field's end (`End` first: the harness's field has no caret to
    /// read until it is placed).
    Right,
    ShiftTab,
    /// Ctrl+K.
    Actions,
    /// Ctrl+Y (⌘Y).
    QuickLook,
    /// Ctrl+R (⌘R).
    Reveal,
    /// Ctrl+Enter (⌘Enter).
    CommandEnter,
    /// Ctrl+. (⌘.).
    CommandPeriod,
}

impl Open {
    /// The real key event `key` is, through the harness's keyboard.
    pub fn press(&mut self, key: Press) {
        let ctrl = [ShortcutKey::Ctrl];
        let h = &mut self.harness;
        match key {
            Press::Up => h.send(Input::key(ShortcutKey::Up)),
            Press::Down => h.send(Input::key(ShortcutKey::Down)),
            Press::Enter => h.send(Input::key(ShortcutKey::Enter)),
            Press::Escape => h.send(Input::key(ShortcutKey::Escape)),
            Press::Space => h.send(Input::key(ShortcutKey::Space)),
            Press::Left => h.send(Input::key(ShortcutKey::Left)),
            Press::Right => {
                h.send(Input::key(ShortcutKey::End));
                h.send(Input::key(ShortcutKey::Right));
            }
            Press::ShiftTab => h.send(Input::chord(&[ShortcutKey::Shift], ShortcutKey::Tab)),
            Press::Actions => h.send(Input::chord(&ctrl, ShortcutKey::Char('k'))),
            Press::QuickLook => h.send(Input::chord(&ctrl, ShortcutKey::Char('y'))),
            Press::Reveal => h.send(Input::chord(&ctrl, ShortcutKey::Char('r'))),
            Press::CommandEnter => h.send(Input::chord(&ctrl, ShortcutKey::Enter)),
            Press::CommandPeriod => h.send(Input::chord(&ctrl, ShortcutKey::Char('.'))),
        }
    }
}

/// The launcher opened with `query` typed, once its rows (or its empty state) have drawn.
pub fn open(query: &str) -> Open {
    open_looking(query, None)
}

/// [`open`], with the pane looking at files as `look` says rather than as the viewer's parts do.
pub fn open_looking(query: &str, look: Option<Arc<dyn Look>>) -> Open {
    open_with(query, LauncherMode::Search, look, |_| {})
}

/// The launcher opened in `mode` with `query` in the field, after `seed` has set the fake
/// services' state (the companion's answers, say) so the first rows already draw it.
pub fn open_in(query: &str, mode: LauncherMode, seed: impl FnOnce(&mut fake::Feeds)) -> Open {
    open_with(query, mode, None, seed)
}

fn open_with(
    query: &str,
    mode: LauncherMode,
    look: Option<Arc<dyn Look>>,
    seed: impl FnOnce(&mut fake::Feeds),
) -> Open {
    let (services, mut feeds, runner) = launcher_support::services_with_runner();
    seed(&mut feeds.bar);
    let LauncherFeeds {
        bar,
        results,
        searches,
    } = feeds;
    let link = LauncherLink::new();
    link.open_in(fake::output().id, query.to_owned(), mode);
    let running = SurfaceHarness::<Launcher>::setup(link.clone(), ())
        .services(services, bar)
        .in_root(in_root(launcher_support::providers(), runner, look))
        .start();
    let SurfaceHarness {
        mut harness,
        feeds: bar,
        ..
    } = running;
    settle(&mut harness, |h| {
        h.count(".ds-row") > 0 || h.count(".ds-palette-empty") > 0
    });
    Open {
        harness,
        feeds: LauncherFeeds {
            bar,
            results,
            searches,
        },
        link,
    }
}

/// Advance until `done` holds (`settle_until`, which fails with the document if it never does);
/// then one more frame's worth.
pub fn settle(harness: &mut Harness, done: impl Fn(&Harness) -> bool) {
    settle_until(harness, done);
    harness.advance(Duration::from_millis(20));
}

/// The rows' titles, in order (marks and all: the text inside each `.ds-row-title`).
pub fn titles(harness: &Harness) -> Vec<String> {
    let html = harness.html();
    html.split("class=\"ds-row-title ds-truncate\"")
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
    let rest = &rest[rest.find("class=\"ds-row-title ds-truncate\"")?..];
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
