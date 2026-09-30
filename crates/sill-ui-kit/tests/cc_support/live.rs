//! The real `ControlCenter` in a `ds_native::Harness` (a Blitz document, no window), inside a
//! quire `Popover` as the bar's popup draws it, on the fake services: for the tests that click
//! and press keys.

use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use tokio::sync::{mpsc, watch};

use dioxus::prelude::*;
use ds::{
    Align, Anchor, Appearance, Ds, Material, Placement, Point, Popover, Px, RootExtent, Side,
};
use ds_native::{Clock, Harness, HarnessConfig, Viewport};
use sill_model::ids::OutputName;
use sill_settings::{FocusMode, ShellFile};
use sill_ui_kit::hooks::PinnedAppearance;
use sill_ui_kit::links::FocusLink;
use sill_ui_kit::menu::{PopupFrame, PopupRoom};
use sill_ui_kit::services::Services;
use sill_ui_kit::settings::SettingsFeed;
use sill_ui_kit::surfaces::control_center::{CcPage, ControlCenter};

use super::fake;

/// What the harness's root provides, and what the panel reported.
#[derive(Clone)]
struct World {
    services: Services,
    settings: SettingsFeed,
    focus: FocusLink,
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

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
}

fn app() -> Element {
    let world = WORLD.with(|w| w.borrow().clone());
    let Some(world) = world else {
        return rsx! {};
    };
    use_context_provider(|| world.services.clone());
    // Not the desktop's portal and appearance file: they answer off-thread.
    use_context_provider(PinnedAppearance::default);
    use_context_provider(|| world.settings.clone());
    use_context_provider(|| world.focus.clone());
    let pages = world.reports.clone();
    let closes = world.reports.clone();
    let closed = EventHandler::new(move |()| {
        if let Ok(mut reports) = closes.lock() {
            reports.push(Report::Closed);
        }
    });
    rsx! {
        style { {CONTROL_CENTER_CSS} }
        PopupFrame { room: PopupRoom { side: 28, top: 8, bottom: 40 },
        Ds { sheet: Some(ds_shell::stylesheet()), appearance: Appearance::default(), material: Material::Popover,
            extent: RootExtent::Popup,
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
    /// The Focus mode the panel reads (Off at the start), and the modes its tile asked for.
    pub focus: watch::Sender<FocusMode>,
    pub focus_sets: mpsc::UnboundedReceiver<FocusMode>,
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
    start_at(services, feeds, 100)
}

/// [`start`] on an output at `scale_percent` (200: the user's 4K monitor at scale 2).
pub fn start_at(services: Services, feeds: fake::Feeds, scale_percent: u16) -> Running {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let (focus, mode) = watch::channel(FocusMode::Off);
    let (set, focus_sets) = mpsc::unbounded_channel();
    WORLD.with(|w| {
        *w.borrow_mut() = Some(World {
            services,
            settings: SettingsFeed::fixed(ShellFile::default()),
            focus: FocusLink::new(mode, set),
            reports: reports.clone(),
        });
    });
    let viewport = Viewport {
        width: 360,
        height: 900,
        scale_percent,
    };
    Running {
        harness: Harness::with_config(app, HarnessConfig::new(viewport).with_clock(Clock::Virtual)),
        feeds,
        focus,
        focus_sets,
        reports,
    }
}
