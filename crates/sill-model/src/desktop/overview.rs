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
