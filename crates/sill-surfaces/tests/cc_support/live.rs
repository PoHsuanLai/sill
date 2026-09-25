//! The real `ControlCenter` in a `ds_native::Harness` (a Blitz document, no window), inside a
//! quire `Popover` as the bar's popup draws it, on the fake services: for the tests that click
//! and press keys.

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use dioxus::prelude::*;
use ds::{Align, Anchor, Appearance, Ds, Material, Placement, Point, Popover, Px, Side};
use ds_native::{Harness, Viewport};
use sill_services::{OutputName, Services};
use sill_settings::{FocusLink, FocusMode, SettingsFeed, ShellFile};
use sill_surfaces::surfaces::control_center::{CcPage, ControlCenter};

use super::fake;

/// What the harness's root provides, and what the panel reported.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    reports: Arc<Mutex<Vec<Report>>>,
}

/// What the panel told its popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Report {
    Page(CcPage),
    Closed,
}

/// The popup document's control center sheet, which the bar's popup root injects.
const CONTROL_CENTER_CSS: &str = include_str!("../../src/style/control_center.css");
/// The popup document's frame sheet.
const POPUP_CSS: &str = include_str!("../../src/style/bar_popup.css");

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    use_context_provider(|| world.settings.clone());
    use_context_provider(|| FocusLink::fixed(FocusMode::Off));
    let pages = world.reports.clone();
    let closes = world.reports.clone();
    let closed = EventHandler::new(move |()| {
        if let Ok(mut reports) = closes.lock() {
            reports.push(Report::Closed);
        }
    });
    rsx! {
        style { {CONTROL_CENTER_CSS} }
        style { {POPUP_CSS} }
        div { class: "sill-popup-frame",
        Ds { appearance: Appearance::default(), material: Material::Popover,
            Popover {
                anchor: Anchor::Point(Point { x: Px(8.0), y: Px(1.0) }),
                placement: Placement::new(Side::Bottom, Align::Start),
                gap: Px(7.0),
                onclose: closed,
                ControlCenter {
                    output: Some(OutputName("DP-1".to_owned())),
                    on_close: closed,
                    on_page: move |page: CcPage| {
                        if let Ok(mut reports) = pages.lock() {
                            reports.push(Report::Page(page));
                        }
                    },
                }
            }
        }
        }
    }
}

/// A running panel, the feeds behind its services and what it reported.
pub struct Running {
    pub harness: Harness,
    pub feeds: fake::Feeds,
    reports: Arc<Mutex<Vec<Report>>>,
}

impl Running {
    /// Everything reported so far.
    pub fn reports(&self) -> Vec<Report> {
        self.reports.lock().map(|r| r.clone()).unwrap_or_default()
    }
}

/// The panel on `services` (their feeds in `feeds`) and the default settings.
pub fn start(services: Services, feeds: fake::Feeds) -> Running {
    let reports = Arc::new(Mutex::new(Vec::new()));
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(ShellFile::default()),
            reports: reports.clone(),
        });
    });
    let viewport = Viewport {
        width: 360,
        height: 900,
        scale_percent: 100,
    };
    Running {
        harness: Harness::new(app, viewport),
        feeds,
        reports,
    }
}
