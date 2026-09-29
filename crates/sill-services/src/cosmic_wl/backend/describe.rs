//! Protocol descriptions into published values: pure, given the keys and output names the
//! caller has already looked up.

use std::collections::HashSet;

use cosmic_client_toolkit::cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1::State as WindowFlag;
use cosmic_client_toolkit::cosmic_protocols::workspace::v2::client::zcosmic_workspace_handle_v2::{State as CosmicFlags, WorkspaceCapabilities as CosmicAbilities};
use cosmic_client_toolkit::toplevel_info::ToplevelInfo;
use cosmic_client_toolkit::wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::{State as WorkspaceFlags, WorkspaceCapabilities as Abilities};
use cosmic_client_toolkit::workspace::Workspace as WorkspaceInfo;
use sill_launcher::WindowKey;
use sill_model::cosmic_wl::{WorkspaceCap, WorkspaceCaps, Activity, Pinning, Urgency};
use sill_model::cosmic_wl::toplevels::{Sizing, Toplevel, WindowState};
use sill_model::cosmic_wl::workspaces::Workspace;
use sill_model::ids::{OutputName, WorkspaceKey};
use sill_model::units::Visibility;

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
        coordinates: info.coordinates.clone(),
        capabilities: capabilities(info.capabilities, info.cosmic_capabilities),
        pinning: pinning(info.cosmic_state),
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

/// The ext-workspace and COSMIC capability bits as one set. COSMIC's `pin` bit is 3, which
/// shares bits with `rename` and `set_tiling_state`, so every flag is tested whole
/// (`contains`), as the thread's own gates do.
fn capabilities(ext: Abilities, cosmic: CosmicAbilities) -> WorkspaceCaps {
    let from_ext = [
        (Abilities::Activate, WorkspaceCap::Activate),
        (Abilities::Deactivate, WorkspaceCap::Deactivate),
        (Abilities::Remove, WorkspaceCap::Remove),
        (Abilities::Assign, WorkspaceCap::Assign),
    ]
    .into_iter()
    .filter(|(bit, _)| ext.contains(*bit));
    let from_cosmic = [
        (CosmicAbilities::Rename, WorkspaceCap::Rename),
        (CosmicAbilities::SetTilingState, WorkspaceCap::SetTiling),
        (CosmicAbilities::Pin, WorkspaceCap::Pin),
        (CosmicAbilities::Move, WorkspaceCap::Move),
    ]
    .into_iter()
    .filter(|(bit, _)| cosmic.contains(*bit));
    from_ext
        .map(|(_, cap)| cap)
        .chain(from_cosmic.map(|(_, cap)| cap))
        .collect()
}

fn pinning(flags: CosmicFlags) -> Pinning {
    if flags.contains(CosmicFlags::Pinned) {
        Pinning::Pinned
    } else {
        Pinning::Unpinned
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        Abilities, CosmicAbilities, CosmicFlags, WindowFlag, WorkspaceFlags, capabilities, pinning,
        sizing, urgency, visibility, window_activity, window_state, workspace_activity,
    };
    use sill_model::cosmic_wl::WorkspaceCap;
    use sill_model::cosmic_wl::toplevels::{Sizing, WindowState};
    use sill_model::cosmic_wl::{Activity, Pinning, Urgency};
    use sill_model::units::Visibility;

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

    #[test]
    fn capability_bits_become_one_set() {
        use WorkspaceCap::{Activate, Assign, Move, Pin, Remove, Rename, SetTiling};
        let cases: &[(&str, Abilities, CosmicAbilities, &[WorkspaceCap])] = &[
            ("none", Abilities::empty(), CosmicAbilities::empty(), &[]),
            (
                "ext only",
                Abilities::Activate.union(Abilities::Remove),
                CosmicAbilities::empty(),
                &[Activate, Remove],
            ),
            (
                "pin (bit value 3) reads as rename and set_tiling too",
                Abilities::Activate.union(Abilities::Assign),
                CosmicAbilities::Rename
                    .union(CosmicAbilities::Pin)
                    .union(CosmicAbilities::Move),
                // pin (3) carries rename (1) and set_tiling_state (2) with it.
                &[Activate, Assign, Rename, SetTiling, Pin, Move],
            ),
            (
                "rename alone is not pin",
                Abilities::empty(),
                CosmicAbilities::Rename,
                &[Rename],
            ),
        ];
        for (name, ext, cosmic, want) in cases {
            assert_eq!(
                capabilities(*ext, *cosmic).iter().collect::<Vec<_>>(),
                *want,
                "{name}"
            );
        }
    }

    #[test]
    fn the_pinned_flag_becomes_pinning() {
        assert_eq!(pinning(CosmicFlags::empty()), Pinning::Unpinned);
        assert_eq!(pinning(CosmicFlags::Pinned), Pinning::Pinned);
    }
}
