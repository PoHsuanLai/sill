//! `companion.*`: the summon and the confirmation (design/22 §3.30). The double tap's window and
//! how long a focused app has to take a summon are `Ms`, plain integers in the file; the service
//! reads them through [`params`], once per load, as the typed values its machines run on.

use ds_core::word::Word;
use ds_settings::Ms;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};
use sill_model::companion::{CompanionParams, SummonParams, TapParams};
use sill_model::units;

pub use sill_model::companion::InField;

/// `companion.summon`: what summons the companion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum SummonKey {
    /// Pressing the command key twice.
    #[default]
    DoubleTapCommand,
    /// No key summons it.
    Off,
}

/// `companion.notify`: when the companion tells the person it needs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum NotifyWhen {
    /// When a task waits for the person.
    #[default]
    NeedsYou,
    /// Never.
    Never,
}

/// The companion's summon and confirmation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "companion", page = Page::Intelligence)]
pub struct CompanionSettings {
    #[settings(
        label = "Press ⌘ twice to ask",
        help = "`double_tap_command` opens the assistant when the command key is pressed twice; `off` leaves the key alone.",
        section = "Assistant"
    )]
    pub summon: SummonKey,
    #[settings(
        label = "In a text field",
        help = "Whether a summon in a text field makes the field the prompt (`prompt`) or opens the launcher (`launcher`).",
        section = "Assistant"
    )]
    pub in_field: InField,
    #[settings(
        label = "Double-tap speed",
        help = "How long the second press of the command key may follow the first.",
        section = "Assistant",
        range = "150..=600",
        unit = "ms",
        advanced
    )]
    pub double_tap_ms: Ms,
    #[settings(
        label = "Time an app has to take a summon",
        help = "How long the focused app has to take a summon into its own prompt before the launcher opens instead.",
        section = "Assistant",
        range = "40..=500",
        unit = "ms",
        advanced
    )]
    pub app_wait_ms: Ms,
    #[settings(
        label = "Notify when it needs you",
        help = "`needs_you` posts a notification when a task waits for you; `never` does not.",
        section = "Assistant"
    )]
    pub notify: NotifyWhen,
    #[settings(
        label = "Confirmation buttons wait",
        help = "How long the buttons of a confirmation wait before they take a press.",
        section = "Confirmations",
        range = "300..=1500",
        unit = "ms",
        advanced
    )]
    pub confirm_arm_ms: Ms,
}

impl Default for CompanionSettings {
    fn default() -> Self {
        CompanionSettings {
            summon: SummonKey::default(),
            in_field: InField::default(),
            double_tap_ms: Ms(350),
            app_wait_ms: Ms(80),
            notify: NotifyWhen::default(),
            confirm_arm_ms: Ms(500),
        }
    }
}

/// What the companion service's machines run on for `settings`, each time held to its key's
/// range so a hand-edited file cannot make the double tap unreachable or the summon wait for
/// seconds.
pub fn params(settings: &CompanionSettings) -> CompanionParams {
    let held = |ms: Ms, low: u16, high: u16| units::Ms(ms.0.clamp(low, high));
    CompanionParams {
        tap: TapParams {
            window: held(settings.double_tap_ms, 150, 600),
        },
        summon: SummonParams {
            app_wait: held(settings.app_wait_ms, 40, 500),
            in_field: settings.in_field,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_designs_and_the_machines_start_on_them() {
        let settings = CompanionSettings::default();
        assert_eq!(
            (settings.double_tap_ms, settings.app_wait_ms),
            (Ms(350), Ms(80))
        );
        assert_eq!(
            params(&settings),
            CompanionParams::default(),
            "the machines' own defaults are the keys' defaults"
        );
    }

    #[test]
    fn a_value_outside_its_range_is_held_to_it() {
        type Case = (&'static str, u16, u16, units::Ms, units::Ms);
        const CASES: &[Case] = &[
            ("both low", 10, 5, units::Ms(150), units::Ms(40)),
            ("both high", 5000, 9000, units::Ms(600), units::Ms(500)),
            ("both inside", 200, 120, units::Ms(200), units::Ms(120)),
        ];
        for (name, tap, wait, want_tap, want_wait) in CASES {
            let settings = CompanionSettings {
                double_tap_ms: Ms(*tap),
                app_wait_ms: Ms(*wait),
                ..CompanionSettings::default()
            };
            let params = params(&settings);
            assert_eq!(params.tap.window, *want_tap, "{name}: tap");
            assert_eq!(params.summon.app_wait, *want_wait, "{name}: wait");
        }
    }

    #[test]
    fn in_field_reaches_the_summon_machine() {
        let settings = CompanionSettings {
            in_field: InField::Launcher,
            ..CompanionSettings::default()
        };
        assert_eq!(params(&settings).summon.in_field, InField::Launcher);
    }
}
