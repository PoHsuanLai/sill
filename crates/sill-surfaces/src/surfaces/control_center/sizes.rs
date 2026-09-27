//! How tall each module and each detail pane is, for the popup's estimate (shell-host never
//! measures the popup, G7), from quire's sizes rather than copies of them (design/29 R9):
//! `ds::CONTROL_CENTER` and `ds::ControlSize`. What sill's own sheet adds (`control_center.css`)
//! is named here. Each is checked against a Harness layout in `tests/cc_estimates.rs`: an
//! estimate covers its box with under 8 px to spare. Pure.

use ds::{CONTROL_CENTER, ControlSize, WholePx};
use sill_settings::ControlCenterModule;

/// A module's estimated height in the grid (its own box; the grid adds the gaps).
pub(crate) fn module_height(module: ControlCenterModule) -> i32 {
    let padded = |content: i32| 2 * whole(CONTROL_CENTER.padding) + content;
    let headed = |content: i32| {
        padded(whole(CONTROL_CENTER.head) + whole(CONTROL_CENTER.head_gap) + content)
    };
    match module {
        // quire's `ModuleTile`: two Large controls tall (56).
        ControlCenterModule::Wifi | ControlCenterModule::Bluetooth | ControlCenterModule::Focus => {
            whole(CONTROL_CENTER.tile)
        }
        // quire's `ModulePanel` round a Regular capsule (64).
        ControlCenterModule::Display
        | ControlCenterModule::KeyboardBrightness
        | ControlCenterModule::Sound => whole(CONTROL_CENTER.level_module()),
        ControlCenterModule::NowPlaying => padded(NOW_PLAYING_BODY),
        ControlCenterModule::Appearance => padded(APPEARANCE_BODY),
        ControlCenterModule::Battery => headed(BATTERY_BODY),
    }
}

/// Now Playing inside its panel's padding: the 48 art and its lines, `.sill-cc-media`'s 6 gap,
/// the position bar with its times (25.8 laid out); 99.8 in all.
const NOW_PLAYING_BODY: i32 = 48 + 6 + 26;
/// quire's compact `AppearancePicker` inside its panel's padding, no header (121 in all).
/// The segmented control is not on the size ladder yet (design/29 §15); when it moves, this
/// moves with it (the test says by how much).
const APPEARANCE_BODY: i32 = 101;
/// Battery under its header: the device rings over their percentages, the time left, the power
/// profile's control (199.2 in all).
const BATTERY_BODY: i32 = 158;

/// A detail pane's height with `list_rows` rows, before the panel's padding: the head, the
/// list (or the empty line), the foot, and `.sill-cc-detail`'s gap between them.
pub(crate) fn detail_height(list_rows: usize) -> i32 {
    let rows = i32::try_from(list_rows).unwrap_or(i32::MAX / LIST_ROW);
    let list = rows.saturating_mul(LIST_ROW).max(empty_line());
    detail_head() + DETAIL_GAP + list + DETAIL_GAP + detail_foot()
}

/// `.sill-cc-detail`'s gap (`--s-6`).
const DETAIL_GAP: i32 = 6;
/// quire's `SettingsRow`: 44 at least (`settings_row.css`; no Rust size yet).
const LIST_ROW: i32 = 44;

/// The head: the back button, the title and the switch, a Large control, in its 2 and 6 of
/// padding (36).
fn detail_head() -> i32 {
    whole(ControlSize::Large.scale().height) + 2 + 6
}

/// The foot: a hairline, 6 above a Regular button (29).
fn detail_foot() -> i32 {
    1 + 6 + whole(ControlSize::Regular.scale().height)
}

/// The empty list's line: 10 above and below a 13 px line at 1.55 (40.2).
fn empty_line() -> i32 {
    2 * 10 + 21
}

/// A quire size as the estimate's pixels.
fn whole(px: WholePx) -> i32 {
    i32::from(px.0)
}

#[cfg(test)]
mod tests {
    use super::{detail_height, module_height};
    use ds::CONTROL_CENTER;
    use sill_settings::ControlCenterModule;

    /// The tile and the level module are quire's own numbers, not copies (R9).
    #[test]
    fn tiles_and_levels_are_quires_sizes() {
        assert_eq!(
            module_height(ControlCenterModule::Wifi),
            i32::from(CONTROL_CENTER.tile.0)
        );
        assert_eq!(
            module_height(ControlCenterModule::Sound),
            i32::from(CONTROL_CENTER.level_module().0)
        );
    }

    #[test]
    fn a_detail_pane_is_its_head_rows_and_foot() {
        // head 36, gap 6, rows 44 each (or the 41 empty line), gap 6, foot 29.
        assert_eq!(detail_height(3), 36 + 6 + 3 * 44 + 6 + 29);
        assert_eq!(detail_height(0), 36 + 6 + 41 + 6 + 29);
    }
}
