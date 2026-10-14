//! The dock's fixtures: four apps, two pins, a badge and windows for the fake services to be fed
//! (`Feeds::apps`, `pins`, `badges`, `windows`); they start empty.

use std::path::PathBuf;

use sill_launcher::{AppId, WindowKey};
use sill_model::apps::{
    App, AppIcon, AppIndex, DbusActivation, DesktopActionEntry, Listing, Terminal,
};
use sill_model::badges::{Badge, Badges};
use sill_model::ids::{OutputName, WorkspaceKey};
use sill_model::pins::DockPins;
use sill_model::units::{Permille, Visibility};
use sill_model::windows::{Activity, Sizing, Toplevel, Toplevels, WindowState};

/// App ids the fixtures use.
pub const TERM: &str = "foot";
pub const FILES: &str = "org.gnome.Nautilus";
pub const SETTINGS: &str = "org.quire.Settings";
pub const FIREFOX: &str = "org.mozilla.firefox";

/// An entry named `name` whose icon is the hicolor theme's, when this machine has it.
fn app(id: &str, name: &str, actions: &[(&str, &str)]) -> App {
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
        categories: Vec::new(),
        keyboard_profile: None,
        keywords: Vec::new(),
        listing: Listing::Shown,
        path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
    }
}

/// Four apps, sorted by id as the index keeps them.
pub fn index() -> AppIndex {
    let mut apps = vec![
        app(FILES, "Example Files", &[]),
        app(SETTINGS, "Settings", &[]),
        app(TERM, "Example Terminal", &[("new-window", "New Window")]),
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
