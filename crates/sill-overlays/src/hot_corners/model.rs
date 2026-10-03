//! What the settings say about the corners: which do something, and what.

use sill_settings::{CornerAction, HotCornerSettings};

use sill_ui_kit::links::Corner;

/// What `corner` does per `settings`.
pub fn corner_action(settings: &HotCornerSettings, corner: Corner) -> CornerAction {
    match corner {
        Corner::TopLeft => settings.top_left,
        Corner::TopRight => settings.top_right,
        Corner::BottomLeft => settings.bottom_left,
        Corner::BottomRight => settings.bottom_right,
    }
}

/// The command `corner` runs when its action is `Command`.
pub fn corner_command(settings: &HotCornerSettings, corner: Corner) -> &str {
    match corner {
        Corner::TopLeft => &settings.top_left_command,
        Corner::TopRight => &settings.top_right_command,
        Corner::BottomLeft => &settings.bottom_left_command,
        Corner::BottomRight => &settings.bottom_right_command,
    }
}

/// The corners that get a surface: every one whose action is not `None`.
pub fn active_corners(settings: &HotCornerSettings) -> Vec<Corner> {
    Corner::ALL
        .into_iter()
        .filter(|corner| corner_action(settings, *corner) != CornerAction::None)
        .collect()
}

#[cfg(test)]
mod tests {
    use sill_settings::{CornerAction, HotCornerSettings};

    use super::{active_corners, corner_action, corner_command};
    use sill_ui_kit::links::Corner;

    #[test]
    fn only_corners_with_an_action_get_a_surface() {
        let mut settings = HotCornerSettings::default();
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
