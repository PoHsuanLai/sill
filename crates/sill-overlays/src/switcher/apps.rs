//! The app switcher's data (design/13 §13.3.5): open windows grouped into apps in
//! most-recently-used order, as macOS's Cmd+Tab shows applications, not
//! windows, and the `windows` commands each of its verbs becomes. Pure: the surface derives
//! the list from `Services.toplevels` and `Services.apps` whenever it shows, and sends the
//! commands through `Services.windows`; there is no task, since the switcher has no state of its
//! own outside the surface's machine (`ds_behaviour::switcher::Switcher`).

use sill_launcher::{AppId, WindowKey};

use sill_model::apps::AppIndex;
use sill_model::windows::{Toplevel, Toplevels, WindowState, WlCommand, app_of};

/// Whether an app has a window on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppShown {
    /// At least one window is not minimized.
    #[default]
    Visible,
    /// Every window is minimized (design/13: still listed).
    AllMinimized,
}

/// One tile: an app and its windows, the most recently active first.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SwitcherApp {
    pub app: AppId,
    pub windows: Vec<WindowKey>,
    pub shown: AppShown,
}

/// `toplevels` (already most recently activated first) grouped by app, in the order each app's
/// most recent window appears; a window is its app's by [`sill_model::windows::app_of`] over `index`, else
/// its raw `app_id` (so an app with no desktop entry still gets a tile); a window with an empty
/// `app_id` is skipped.
pub fn switcher_apps(toplevels: &Toplevels, index: &AppIndex) -> Vec<SwitcherApp> {
    toplevels
        .windows
        .iter()
        .filter(|window| !window.app_id.trim().is_empty())
        .map(|window| (app_of(window, index).id(), window))
        .fold(Vec::new(), |mut apps: Vec<SwitcherApp>, (app, window)| {
            match apps.iter_mut().find(|known| known.app == app) {
                Some(known) => {
                    known.windows.push(window.key);
                    known.shown = merged(known.shown, shown_of(window));
                }
                None => apps.push(SwitcherApp {
                    app,
                    windows: vec![window.key],
                    shown: shown_of(window),
                }),
            }
            apps
        })
}

fn shown_of(window: &Toplevel) -> AppShown {
    match window.window {
        WindowState::Normal => AppShown::Visible,
        WindowState::Minimized => AppShown::AllMinimized,
    }
}

/// An app is visible when any of its windows is.
fn merged(a: AppShown, b: AppShown) -> AppShown {
    match (a, b) {
        (AppShown::AllMinimized, AppShown::AllMinimized) => AppShown::AllMinimized,
        _ => AppShown::Visible,
    }
}

/// Release on an app (design/13 "Release"): unminimize and activate its most recent window;
/// macOS brings only that one forward. `Unminimize` first when every window is minimized.
pub fn activate(app: &SwitcherApp) -> Vec<WlCommand> {
    let Some(&first) = app.windows.first() else {
        return Vec::new();
    };
    match app.shown {
        AppShown::Visible => vec![WlCommand::Activate(first)],
        AppShown::AllMinimized => vec![WlCommand::Unminimize(first), WlCommand::Activate(first)],
    }
}

/// Q: ask every window of the app to close.
pub fn quit(app: &SwitcherApp) -> Vec<WlCommand> {
    app.windows.iter().copied().map(WlCommand::Close).collect()
}

/// H: minimize every window of the app.
pub fn hide(app: &SwitcherApp) -> Vec<WlCommand> {
    app.windows
        .iter()
        .copied()
        .map(WlCommand::Minimize)
        .collect()
}

#[cfg(test)]
mod tests {
    use sill_launcher::{AppId, WindowKey};

    use super::{AppShown, SwitcherApp, activate, hide, quit, switcher_apps};
    use sill_model::apps::AppIndex;
    use sill_model::windows::{Activity, Sizing, Toplevel, Toplevels, WindowState, WlCommand};

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
