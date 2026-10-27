//! What a click on a running app asks the compositor for (design/10 §10.3.6):
//! activate its most recent window (which also unminimizes), and the Option-click hides. Pure
//! over the toplevel list, whose order is most recently activated first.

use sill_model::ids::WorkspaceKey;
use sill_model::windows::{Toplevel, Toplevels, WlCommand};

use crate::dock::machine::ActivateMode;

/// The commands for `mode` on an app whose windows are `mine` (most recent first), among
/// `all`, with `workspace` the output's active workspace when known. `Reopen` is a launch,
/// not a compositor command, and yields nothing here.
pub fn activation(
    mode: ActivateMode,
    mine: &[&Toplevel],
    all: &Toplevels,
    workspace: Option<WorkspaceKey>,
) -> Vec<WlCommand> {
    let Some(recent) = mine.first() else {
        return Vec::new();
    };
    let raise = WlCommand::Activate(recent.key);
    match mode {
        ActivateMode::Raise | ActivateMode::Unminimize => vec![raise],
        ActivateMode::Reopen => Vec::new(),
        ActivateMode::HideCurrent => {
            let front = all.active();
            let hidden = front
                .filter(|f| !mine.iter().any(|m| m.key == f.key))
                .map(|f| minimize_app(all, &f.app_id))
                .unwrap_or_default();
            std::iter::once(raise).chain(hidden).collect()
        }
        ActivateMode::HideOthers => {
            let others = all
                .windows
                .iter()
                .filter(|w| !mine.iter().any(|m| m.key == w.key))
                .filter(|w| on(w, workspace))
                .map(|w| WlCommand::Minimize(w.key));
            std::iter::once(raise).chain(others).collect()
        }
    }
}

/// Whether `window` is on `workspace` (every window is, when the workspace is unknown).
fn on(window: &Toplevel, workspace: Option<WorkspaceKey>) -> bool {
    workspace.is_none_or(|key| window.workspaces.contains(&key))
}

fn minimize_app(all: &Toplevels, app_id: &str) -> Vec<WlCommand> {
    all.windows
        .iter()
        .filter(|w| w.app_id == app_id)
        .map(|w| WlCommand::Minimize(w.key))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::activation;
    use crate::dock::machine::ActivateMode;
    use sill_launcher::WindowKey;
    use sill_model::ids::WorkspaceKey;
    use sill_model::windows::{Activity, Sizing, Toplevel, Toplevels, WindowState, WlCommand};

    fn window(key: u64, app: &str, activity: Activity, workspace: u64) -> Toplevel {
        Toplevel {
            identifier: String::new(),
            key: WindowKey(key),
            app_id: app.to_owned(),
            title: String::new(),
            activity,
            window: WindowState::Normal,
            sizing: Sizing::Floating,
            workspaces: vec![WorkspaceKey(workspace)],
            outputs: Vec::new(),
        }
    }

    #[test]
    fn each_mode_asks_for_its_windows() {
        use Activity::{Active, Inactive};
        use WlCommand::{Activate, Minimize};
        let all = Toplevels {
            windows: vec![
                window(1, "files", Active, 1),
                window(2, "term", Inactive, 1),
                window(3, "term", Inactive, 2),
                window(4, "term", Inactive, 1),
                window(5, "files", Inactive, 1),
            ],
        };
        let term: Vec<&Toplevel> = all.windows.iter().filter(|w| w.app_id == "term").collect();
        let ws = Some(WorkspaceKey(1));
        let k = WindowKey;
        let cases = [
            (ActivateMode::Raise, vec![Activate(k(2))]),
            (ActivateMode::Unminimize, vec![Activate(k(2))]),
            (ActivateMode::Reopen, vec![]),
            (
                ActivateMode::HideCurrent,
                vec![Activate(k(2)), Minimize(k(1)), Minimize(k(5))],
            ),
            (
                ActivateMode::HideOthers,
                vec![Activate(k(2)), Minimize(k(1)), Minimize(k(5))],
            ),
        ];
        for (mode, want) in cases {
            assert_eq!(activation(mode, &term, &all, ws), want, "{mode:?}");
        }
    }
}
