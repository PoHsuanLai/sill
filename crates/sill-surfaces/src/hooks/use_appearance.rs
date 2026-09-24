//! `use_appearance`: quire's appearance, from `ds-settings` (plan hooks table: "use_appearance
//! (from ds)").

use dioxus::prelude::ReadSignal;
use ds_settings::{AppName, Environment, use_environment};

/// `quire/appearance.toml` and the settings portal, live.
pub fn use_appearance() -> ReadSignal<Environment> {
    use_environment(AppName::QUIRE)
}
