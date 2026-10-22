//! The requests that open the desktop's overviews from the shell's own verbs.

use casement_protocols::vocab::{OverviewMode, Toggle};

use super::DesktopCommand;

/// Open Mission Control (`sill workspace overview`, the hot corner that shows the Spaces).
pub fn open_mission_control() -> DesktopCommand {
    DesktopCommand::Overview {
        mode: OverviewMode::MissionControl,
        toggle: Toggle::On,
    }
}

/// Open App Exposé, the focused app's windows spread out (the `app_windows` hot corner).
pub fn open_app_expose() -> DesktopCommand {
    DesktopCommand::Overview {
        mode: OverviewMode::AppExpose,
        toggle: Toggle::On,
    }
}
