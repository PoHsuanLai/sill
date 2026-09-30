//! The Appearance module: quire's `AppearancePicker` (the one picker, plan) in its compact
//! layout, which fits the module's width, its value from
//! `use_appearance`, a change written to `quire/appearance.toml` through `ds-settings` (the file
//! quire's own settings watch then publishes to every surface).

use dioxus::prelude::*;
use ds::{Appearance, AppearancePicker, PickerLayout};
use ds_settings::{AppName, AppearanceFile, ConfigRoot, Store};
use ds_shell::ModulePanel;

use super::cc_module::{CcCx, CcModule, Span};
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
    let store = Store::new(ConfigRoot::Xdg, AppName::QUIRE);
    let file = with_appearance(store.load::<AppearanceFile>().value, next);
    if let Err(error) = store.save(&file) {
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

/// The Appearance module as the control center draws it: a row, never a bar item.
pub(crate) struct AppearanceCc;

impl CcModule for AppearanceCc {
    fn span(&self) -> Span {
        Span::Row
    }

    fn tile(&self, _cx: &CcCx) -> Element {
        rsx! { AppearanceModule {} }
    }
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
            motion: Motion::Reduced,
        };
        let file = with_appearance(AppearanceFile::default(), next);
        assert_eq!(file.appearance.appearance(), next);
        assert_eq!(file.icons, AppearanceFile::default().icons);
    }
}
