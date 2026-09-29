//! The catcher closes the launcher on a press anywhere on it (design/06 §20.2), in a
//! `ds_native::Harness`.

mod launcher_harness;

use std::cell::RefCell;
use std::time::Duration;

use dioxus::prelude::*;
use ds_native::{Harness, Viewport};
use launcher_harness::launcher_support::{self, fake};
use launcher_harness::virtual_clock;
use sill_settings::{SettingsFeed, ShellFile};
use sill_ui_kit::LauncherCatcher;
use sill_ui_kit::links::LauncherLink;
use sill_ui_kit::services::Services;

thread_local! {
    static WORLD: RefCell<Option<(Services, LauncherLink)>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let Some((services, link)) = WORLD.with(|w| w.borrow().clone()) else {
        return rsx! {};
    };
    use_context_provider(|| services.clone());
    use_context_provider(|| SettingsFeed::fixed(ShellFile::default()));
    use_context_provider(|| link.clone());
    rsx! {
        LauncherCatcher { output: fake::output() }
    }
}

#[test]
fn a_press_on_the_catcher_closes_the_launcher() {
    let (services, _feeds) = launcher_support::services();
    let link = LauncherLink::new();
    link.toggle(fake::output().id);
    WORLD.with(|w| *w.borrow_mut() = Some((services, link.clone())));
    let viewport = Viewport {
        width: 960,
        height: 540,
        scale_percent: 100,
    };
    let mut harness = Harness::with_config(app, virtual_clock(viewport));
    harness.advance(Duration::from_millis(20));
    assert!(link.now().opening().is_some());
    harness.click(ds::Point {
        x: ds::Px(20.0),
        y: ds::Px(500.0),
    });
    harness.advance(Duration::from_millis(20));
    assert!(link.now().opening().is_none(), "{}", harness.html());
}
