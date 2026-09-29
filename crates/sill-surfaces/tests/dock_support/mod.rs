//! Fake services for the dock's tests and `examples/dock_demo.rs`: the bar's fakes, with an
//! app index, pins and badges of the dock's own whose senders the caller holds.

#![allow(dead_code)] // Each test binary uses its own subset.

#[path = "../support/fake.rs"]
pub mod fake;

use std::path::PathBuf;

use sill_launcher::{AppId, WindowKey};
use sill_model::apps::{
    App, AppIcon, AppIndex, DbusActivation, DesktopActionEntry, Listing, Terminal,
};
use sill_model::badges::{Badge, Badges};
use sill_model::cosmic_wl::{Activity, Sizing, Toplevel, Toplevels, WindowState};
use sill_model::ids::{OutputName, WorkspaceKey};
use sill_model::pins::DockPins;
use sill_model::units::{Permille, Visibility};
use sill_services::Services;
use tokio::sync::{mpsc, watch};

pub const TERM: &str = "com.system76.CosmicTerm";
pub const FILES: &str = "com.system76.CosmicFiles";
pub const SETTINGS: &str = "com.system76.CosmicSettings";
pub const FIREFOX: &str = "org.mozilla.firefox";

/// The dock's senders beside the bar's.
pub struct DockFeeds {
    pub bar: fake::Feeds,
    pub apps: watch::Sender<AppIndex>,
    pub pins: watch::Sender<DockPins>,
    pub badges: watch::Sender<Badges>,
    pub pin_events: mpsc::UnboundedReceiver<sill_model::pins::PinEvent>,
}

/// An entry named `name` whose icon is the hicolor theme's, when this machine has it.
pub fn app(id: &str, name: &str, actions: &[(&str, &str)]) -> App {
    let icon = ["scalable", "256x256", "128x128"]
        .iter()
        .map(|size| PathBuf::from(format!("/usr/share/icons/hicolor/{size}/apps/{id}.svg")))
        .find(|path| path.is_file())
        .map_or(AppIcon::None, AppIcon::File);
    App {
        id: AppId(id.to_owned()),
        name: name.to_owned(),
        generic_name: String::new(),
        comment: String::new(),
        exec: id.to_owned(),
        icon,
        startup_wm_class: None,
        flatpak: None,
        terminal: Terminal::No,
        activation: DbusActivation::Exec,
        actions: actions
            .iter()
            .map(|(id, name)| DesktopActionEntry {
                id: (*id).to_owned(),
                name: (*name).to_owned(),
                exec: String::new(),
            })
            .collect(),
        mime_types: Vec::new(),
        keywords: Vec::new(),
        listing: Listing::Shown,
        path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
    }
}

/// Four apps, sorted by id as the index keeps them.
pub fn index() -> AppIndex {
    let mut apps = vec![
        app(FILES, "COSMIC Files", &[]),
        app(SETTINGS, "COSMIC Settings", &[]),
        app(TERM, "COSMIC Terminal", &[("new-window", "New Window")]),
        app(FIREFOX, "Firefox", &[]),
    ];
    apps.sort_by(|a, b| a.id.cmp(&b.id));
    AppIndex { apps }
}

/// Files and Terminal pinned.
pub fn pins() -> DockPins {
    DockPins {
        pinned: vec![AppId(FILES.to_owned()), AppId(TERM.to_owned())],
    }
}

/// Terminal (pinned) active with two windows, Firefox (not pinned) with one, on `output`.
pub fn toplevels_on(output: &str) -> Toplevels {
    let window = |key: u64, app: &str, activity| Toplevel {
        key: WindowKey(key),
        app_id: app.to_owned(),
        title: format!("{app} {key}"),
        activity,
        window: WindowState::Normal,
        sizing: Sizing::Floating,
        workspaces: vec![WorkspaceKey(2)],
        outputs: vec![OutputName(output.to_owned())],
    };
    Toplevels {
        windows: vec![
            window(10, TERM, Activity::Active),
            window(12, FIREFOX, Activity::Inactive),
            window(11, TERM, Activity::Inactive),
        ],
    }
}

/// Firefox shows a count of 7 and 40 % progress.
pub fn badges() -> Badges {
    let mut badges = Badges::default();
    badges.apps.insert(
        AppId(FIREFOX.to_owned()),
        Badge {
            count: 7,
            count_visibility: Visibility::Shown,
            progress: Permille(400),
            progress_visibility: Visibility::Shown,
            ..Badge::default()
        },
    );
    badges
}

/// The bar's fakes with the dock's index, pins, badges and windows.
pub fn services() -> (Services, DockFeeds) {
    let (mut services, bar) = fake::services();
    bar.cosmic_wl
        .send_modify(|wl| wl.toplevels = toplevels_on("DP-1"));
    let (apps_tx, apps) = watch::channel(index());
    let (pins_tx, pins) = watch::channel(pins());
    let (badges_tx, badges) = watch::channel(badges());
    let (pins_control, pin_events) = sill_services::Control::channel();
    services.apps.state = apps;
    services.pins.state = pins;
    services.badges.state = badges;
    services.pins.control = pins_control;
    (
        services,
        DockFeeds {
            bar,
            apps: apps_tx,
            pins: pins_tx,
            badges: badges_tx,
            pin_events,
        },
    )
}
