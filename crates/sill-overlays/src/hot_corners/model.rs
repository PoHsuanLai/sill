//! What the settings say about the corners: which do something, and what.

use ds::base::word::Word;
use sill_settings::{CornerBindings, HotCornerSettings};

use sill_ui_kit::links::Corner;

/// What `corner` does per `settings`: its bindings, one per set of held keys.
pub fn corner_bindings(settings: &HotCornerSettings, corner: Corner) -> &CornerBindings {
    match corner {
        Corner::TopLeft => &settings.top_left,
        Corner::TopRight => &settings.top_right,
        Corner::BottomLeft => &settings.bottom_left,
        Corner::BottomRight => &settings.bottom_right,
    }
}

/// The corners that get a surface: every one with at least one binding.
pub fn active_corners(settings: &HotCornerSettings) -> Vec<Corner> {
    Corner::ALL
        .iter()
        .copied()
        .filter(|corner| !corner_bindings(settings, *corner).0.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use sill_settings::{
        CornerAction, CornerBinding, CornerBindings, CornerDo, CornerKeys, HotCornerSettings,
    };

    use super::{active_corners, corner_bindings};
    use sill_ui_kit::links::Corner;

    /// One binding, for no keys, that does `action`.
    fn plain(action: CornerAction) -> CornerBindings {
        CornerBindings(vec![CornerBinding {
            keys: CornerKeys::default(),
            action: CornerDo::Desktop(action),
        }])
    }

    #[test]
    fn only_corners_with_a_binding_get_a_surface() {
        let mut settings = HotCornerSettings::default();
        assert_eq!(active_corners(&settings), Vec::new());
        settings.top_left = plain(CornerAction::Launcher);
        settings.bottom_right = CornerBindings(vec![CornerBinding {
            keys: CornerKeys::default(),
            action: CornerDo::Command("foot".to_owned()),
        }]);
        assert_eq!(
            active_corners(&settings),
            vec![Corner::TopLeft, Corner::BottomRight]
        );
        assert_eq!(
            corner_bindings(&settings, Corner::TopLeft),
            &plain(CornerAction::Launcher)
        );
        assert_eq!(corner_bindings(&settings, Corner::TopRight).0, Vec::new());
    }
}
