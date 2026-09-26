//! The app switcher's data (design/13 §13.3.5, M11; FINDINGS "M11 freeze" F548): open windows
//! grouped into apps in most-recently-used order, as macOS's Cmd+Tab shows applications, not
//! windows, and the `cosmic_wl` commands each of its verbs becomes. Pure: the surface derives
//! the list from `Services.toplevels` and `Services.apps` whenever it shows, and sends the
//! commands through `Services.wl`; there is no task, since the switcher has no state of its
//! own outside the surface's machine (`sill-surfaces` `surfaces::switcher::machine`).

use serde::{Deserialize, Serialize};
use sill_launcher::{AppId, WindowKey};

use crate::apps::AppIndex;
use crate::cosmic_wl::{Toplevels, WlCommand};

/// Whether an app has a window on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppShown {
    /// At least one window is not minimized.
    #[default]
    Visible,
    /// Every window is minimized (design/13: still listed).
    AllMinimized,
}

/// One tile: an app and its windows, the most recently active first.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SwitcherApp {
    pub app: AppId,
    pub windows: Vec<WindowKey>,
    pub shown: AppShown,
}

/// `toplevels` (already most recently activated first) grouped by app, in the order each app's
/// most recent window appears; a window is its app's by [`crate::match_app`] over `index`, else
/// its raw `app_id` (so an app with no desktop entry still gets a tile); a window with an empty
/// `app_id` is skipped.
pub fn switcher_apps(toplevels: &Toplevels, index: &AppIndex) -> Vec<SwitcherApp> {
    todo!(
        "M11 fill: c-switcher ({} windows, {} apps)",
        toplevels.windows.len(),
        index.apps.len()
    )
}

/// Release on an app (design/13 "Release"): unminimize and activate its most recent window;
/// macOS brings only that one forward. `Unminimize` first when every window is minimized.
pub fn activate(app: &SwitcherApp) -> Vec<WlCommand> {
    todo!("M11 fill: c-switcher ({app:?})")
}

/// Q: ask every window of the app to close.
pub fn quit(app: &SwitcherApp) -> Vec<WlCommand> {
    todo!("M11 fill: c-switcher ({app:?})")
}

/// H: minimize every window of the app.
pub fn hide(app: &SwitcherApp) -> Vec<WlCommand> {
    todo!("M11 fill: c-switcher ({app:?})")
}

#[cfg(test)]
mod tests {
    use sill_launcher::{AppId, WindowKey};

    use super::{AppShown, SwitcherApp, activate, hide, quit, switcher_apps};
    use crate::apps::AppIndex;
    use crate::cosmic_wl::{Activity, Sizing, Toplevel, Toplevels, WindowState, WlCommand};

    fn window(key: u64, app_id: &str, window: WindowState) -> Toplevel {
        Toplevel {
            key: WindowKey(key),
            app_id: app_id.to_owned(),
            title: format!("{app_id} {key}"),
            activity: Activity::Inactive,
            window,
            sizing: Sizing::Floating,
            workspaces: Vec::new(),
            outputs: Vec::new(),
        }
    }

    fn app(id: &str, windows: &[u64], shown: AppShown) -> SwitcherApp {
        SwitcherApp {
            app: AppId(id.to_owned()),
            windows: windows.iter().map(|&k| WindowKey(k)).collect(),
            shown,
        }
    }

    #[test]
    #[ignore = "M11 fill: c-switcher"]
    fn windows_group_into_apps_in_mru_order() {
        let toplevels = Toplevels {
            windows: vec![
                window(1, "foot", WindowState::Normal),
                window(2, "firefox", WindowState::Normal),
                window(3, "foot", WindowState::Normal),
                window(4, "mail", WindowState::Minimized),
                window(5, "", WindowState::Normal),
                window(6, "firefox", WindowState::Minimized),
            ],
        };
        assert_eq!(
            switcher_apps(&toplevels, &AppIndex::default()),
            vec![
                app("foot", &[1, 3], AppShown::Visible),
                app("firefox", &[2, 6], AppShown::Visible),
                app("mail", &[4], AppShown::AllMinimized),
            ]
        );
    }

    #[test]
    #[ignore = "M11 fill: c-switcher"]
    fn each_verb_as_commands() {
        let foot = app("foot", &[1, 3], AppShown::Visible);
        let mail = app("mail", &[4], AppShown::AllMinimized);
        assert_eq!(activate(&foot), vec![WlCommand::Activate(WindowKey(1))]);
        assert_eq!(
            activate(&mail),
            vec![
                WlCommand::Unminimize(WindowKey(4)),
                WlCommand::Activate(WindowKey(4)),
            ]
        );
        assert_eq!(
            quit(&foot),
            vec![
                WlCommand::Close(WindowKey(1)),
                WlCommand::Close(WindowKey(3))
            ]
        );
        assert_eq!(
            hide(&foot),
            vec![
                WlCommand::Minimize(WindowKey(1)),
                WlCommand::Minimize(WindowKey(3)),
            ]
        );
    }
}
