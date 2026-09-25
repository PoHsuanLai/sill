//! Hot corners (design/20 §1.16; macOS's Hot Corners; M10 lane d): a transparent square of
//! `hot_corners.size_px` in each corner whose action is not `None`, on every output. The pointer
//! resting in one for `hot_corners.dwell_ms` (with `hot_corners.modifier` held, when one is
//! set) fires it once; it fires again only after the pointer has left and `rearm_ms` has passed
//! ([`machine::step`]). A firing is written to the [`CornerLink`]; the daemon carries out the
//! action (`crates/sill/src/corners.rs`), since opening the launcher, the notification center
//! or the control center, locking, minimising windows and running a command are its links and
//! services, not a surface's.
//!
//! Never painted: the root draws one transparent element that takes the pointer, no `Ds`, no
//! material, no blur; the compositor still delivers pointer enter and leave to it.

pub mod layer;
pub mod link;
pub mod machine;

pub use layer::corner_layer;
pub use link::{CornerHit, CornerLink};
pub use machine::{CornerIn, CornerMachine, CornerOut, CornerParams, KeyState, Mods, step};

use dioxus::prelude::*;
use shell_host::OutputInfo;
use sill_settings::{CornerAction, HotCornersSettings};

/// A screen corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    pub const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];

    /// `top-left`: the namespace's word.
    pub fn word(self) -> &'static str {
        match self {
            Corner::TopLeft => "top-left",
            Corner::TopRight => "top-right",
            Corner::BottomLeft => "bottom-left",
            Corner::BottomRight => "bottom-right",
        }
    }
}

/// What `corner` does per `settings`.
pub fn corner_action(settings: &HotCornersSettings, corner: Corner) -> CornerAction {
    match corner {
        Corner::TopLeft => settings.top_left,
        Corner::TopRight => settings.top_right,
        Corner::BottomLeft => settings.bottom_left,
        Corner::BottomRight => settings.bottom_right,
    }
}

/// The command `corner` runs when its action is `Command`.
pub fn corner_command(settings: &HotCornersSettings, corner: Corner) -> &str {
    match corner {
        Corner::TopLeft => &settings.top_left_command,
        Corner::TopRight => &settings.top_right_command,
        Corner::BottomLeft => &settings.bottom_left_command,
        Corner::BottomRight => &settings.bottom_right_command,
    }
}

/// The corners that get a surface: every one whose action is not `None`.
pub fn active_corners(settings: &HotCornersSettings) -> Vec<Corner> {
    Corner::ALL
        .into_iter()
        .filter(|corner| corner_action(settings, *corner) != CornerAction::None)
        .collect()
}

/// `corner` of `output`. Draws nothing until lane d fills it (then: one transparent element
/// feeding `step` on pointer enter and leave, quire timers for the dwell and the re-arm, and
/// `CornerLink::hit` on `Fire`).
#[component]
pub fn HotCorner(output: OutputInfo, corner: Corner) -> Element {
    let _ = (output, corner);
    rsx! {}
}

#[cfg(test)]
mod tests {
    use sill_settings::{CornerAction, HotCornersSettings};

    use super::{Corner, active_corners, corner_action, corner_command};

    #[test]
    fn only_corners_with_an_action_get_a_surface() {
        let mut settings = HotCornersSettings::default();
        assert_eq!(active_corners(&settings), Vec::new());
        settings.top_left = CornerAction::Launcher;
        settings.bottom_right = CornerAction::Command;
        settings.bottom_right_command = "foot".to_owned();
        assert_eq!(
            active_corners(&settings),
            vec![Corner::TopLeft, Corner::BottomRight]
        );
        assert_eq!(
            corner_action(&settings, Corner::TopLeft),
            CornerAction::Launcher
        );
        assert_eq!(corner_command(&settings, Corner::BottomRight), "foot");
        assert_eq!(corner_command(&settings, Corner::TopRight), "");
    }
}
