//! The Appearance module: quire's `AppearancePicker` (the one picker, plan) in its compact
//! layout, which fits the module's width, its value from
//! `use_appearance`, a change written to `quire/appearance.toml` through `ds-settings` (the file
//! quire's own settings watch then publishes to every surface).

use dioxus::prelude::*;
use ds::{Appearance, AppearancePicker, ModulePanel, PickerLayout};
use ds_settings::{AppName, AppearanceFile, config_dir};

use crate::hooks::use_appearance;
use crate::surfaces::control_center::modules::use_plate;

/// The Appearance module.
#[component]
pub fn AppearanceModule() -> Element {
    let environment = use_appearance()();
    rsx! {
        ModulePanel { plate: use_plate(),
            AppearancePicker {
                value: environment.settings.appearance.appearance(),
                system: environment.system,
                layout: PickerLayout::Compact,
                onchange: move |next: Appearance| save(next),
            }
        }
    }
}

/// Write `next` into `quire/appearance.toml`, keeping everything else in the file. A file that
/// cannot be written leaves the appearance as it was, which the picker then shows.
fn save(next: Appearance) {
    let Some(dir) = config_dir(AppName::QUIRE) else {
        eprintln!("sill: no config directory for quire's appearance");
        return;
    };
    let file = with_appearance(ds_settings::appearance_file::load(&dir), next);
    if let Err(error) = ds_settings::appearance_file::save(&dir, &file) {
        eprintln!("sill: writing the appearance: {error}");
    }
}

/// `file` with `next`'s theme, accent and motion.
fn with_appearance(file: AppearanceFile, next: Appearance) -> AppearanceFile {
    let mut file = file;
    file.appearance.theme = next.theme;
    file.appearance.accent = next.accent;
    file.appearance.motion_level = next.motion;
    file
}

#[cfg(test)]
mod tests {
    use super::with_appearance;
    use ds::Word;
    use ds::{Accent, Appearance, Motion, Theme};
    use ds_settings::AppearanceFile;

    #[test]
    fn a_pick_changes_only_the_three_fields() {
        let next = Appearance {
            theme: Theme::Dark,
            accent: Accent::ALL[1],
            motion: Motion::Calm,
        };
        let file = with_appearance(AppearanceFile::default(), next);
        assert_eq!(file.appearance.appearance(), next);
        assert_eq!(file.icons, AppearanceFile::default().icons);
    }
}
