//! Protocol descriptions into published values: pure, given the keys and output names the
//! caller has already looked up.

use std::collections::HashSet;

use cosmic_client_toolkit::cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1::State as WindowFlag;
use cosmic_client_toolkit::toplevel_info::ToplevelInfo;
use cosmic_client_toolkit::wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::State as WorkspaceFlags;
use cosmic_client_toolkit::workspace::Workspace as WorkspaceInfo;
use sill_launcher::WindowKey;

use crate::cosmic_wl::ids::{Activity, OutputName, Urgency, WorkspaceKey};
use crate::cosmic_wl::toplevels::{Sizing, Toplevel, WindowState};
use crate::cosmic_wl::workspaces::Workspace;
use crate::units::Visibility;

/// The toplevel `info` describes, under `key`, on `workspaces` and `outputs`.
pub(super) fn toplevel(
    key: WindowKey,
    info: &ToplevelInfo,
    workspaces: Vec<WorkspaceKey>,
    outputs: Vec<OutputName>,
) -> Toplevel {
    Toplevel {
        key,
        app_id: info.app_id.clone(),
        title: info.title.clone(),
        activity: window_activity(&info.state),
        window: window_state(&info.state),
        sizing: sizing(&info.state),
        workspaces,
        outputs,
    }
}

/// The workspace `info` describes, under `key`.
pub(super) fn workspace(key: WorkspaceKey, info: &WorkspaceInfo) -> Workspace {
    Workspace {
        key,
        name: info.name.clone(),
        stable_id: info.id.clone().unwrap_or_default(),
        activity: workspace_activity(info.state),
        urgency: urgency(info.state),
        visibility: visibility(info.state),
    }
}

fn window_activity(flags: &HashSet<WindowFlag>) -> Activity {
    if flags.contains(&WindowFlag::Activated) {
        Activity::Active
    } else {
        Activity::Inactive
    }
}

fn window_state(flags: &HashSet<WindowFlag>) -> WindowState {
    if flags.contains(&WindowFlag::Minimized) {
        WindowState::Minimized
    } else {
        WindowState::Normal
    }
}

/// Fullscreen wins over maximized: a maximized window made fullscreen keeps both flags.
fn sizing(flags: &HashSet<WindowFlag>) -> Sizing {
    if flags.contains(&WindowFlag::Fullscreen) {
        Sizing::Fullscreen
    } else if flags.contains(&WindowFlag::Maximized) {
        Sizing::Maximized
    } else {
        Sizing::Floating
    }
}

fn workspace_activity(flags: WorkspaceFlags) -> Activity {
    if flags.contains(WorkspaceFlags::Active) {
        Activity::Active
    } else {
        Activity::Inactive
    }
}

fn urgency(flags: WorkspaceFlags) -> Urgency {
    if flags.contains(WorkspaceFlags::Urgent) {
        Urgency::Urgent
    } else {
        Urgency::Calm
    }
}

fn visibility(flags: WorkspaceFlags) -> Visibility {
    if flags.contains(WorkspaceFlags::Hidden) {
        Visibility::Hidden
    } else {
        Visibility::Shown
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        WindowFlag, WorkspaceFlags, sizing, urgency, visibility, window_activity, window_state,
        workspace_activity,
    };
    use crate::cosmic_wl::ids::{Activity, Urgency};
    use crate::cosmic_wl::toplevels::{Sizing, WindowState};
    use crate::units::Visibility;

    fn flags(of: &[WindowFlag]) -> HashSet<WindowFlag> {
        of.iter().copied().collect()
    }

    #[test]
    fn window_flags_become_the_three_words() {
        use WindowFlag::{Activated, Fullscreen, Maximized, Minimized, Sticky};
        let cases: &[(&str, &[WindowFlag], Activity, WindowState, Sizing)] = &[
            (
                "none",
                &[],
                Activity::Inactive,
                WindowState::Normal,
                Sizing::Floating,
            ),
            (
                "focused",
                &[Activated],
                Activity::Active,
                WindowState::Normal,
                Sizing::Floating,
            ),
            (
                "minimized",
                &[Minimized],
                Activity::Inactive,
                WindowState::Minimized,
                Sizing::Floating,
            ),
            (
                "maximized",
                &[Maximized],
                Activity::Inactive,
                WindowState::Normal,
                Sizing::Maximized,
            ),
            (
                "fullscreen over maximized",
                &[Maximized, Fullscreen, Activated],
                Activity::Active,
                WindowState::Normal,
                Sizing::Fullscreen,
            ),
            (
                "sticky says nothing here",
                &[Sticky],
                Activity::Inactive,
                WindowState::Normal,
                Sizing::Floating,
            ),
        ];
        for (name, given, activity, window, size) in cases {
            let set = flags(given);
            assert_eq!(window_activity(&set), *activity, "{name}");
            assert_eq!(window_state(&set), *window, "{name}");
            assert_eq!(sizing(&set), *size, "{name}");
        }
    }

    #[test]
    fn workspace_flags_become_the_three_words() {
        let cases: &[(&str, WorkspaceFlags, Activity, Urgency, Visibility)] = &[
            (
                "none",
                WorkspaceFlags::empty(),
                Activity::Inactive,
                Urgency::Calm,
                Visibility::Shown,
            ),
            (
                "active",
                WorkspaceFlags::Active,
                Activity::Active,
                Urgency::Calm,
                Visibility::Shown,
            ),
            (
                "urgent and hidden",
                WorkspaceFlags::Urgent.union(WorkspaceFlags::Hidden),
                Activity::Inactive,
                Urgency::Urgent,
                Visibility::Hidden,
            ),
        ];
        for (name, given, activity, urgent, shown) in cases {
            assert_eq!(workspace_activity(*given), *activity, "{name}");
            assert_eq!(urgency(*given), *urgent, "{name}");
            assert_eq!(visibility(*given), *shown, "{name}");
        }
    }
}
