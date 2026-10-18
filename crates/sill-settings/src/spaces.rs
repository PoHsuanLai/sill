//! `spaces.*` (design/22 §3.14; design/21): the defaults a workspace without a stored
//! `SpaceLook` gets. The per-workspace looks are state in `quire/spaces.json`, not here. All
//! Advanced.

use ds_settings::Count;
use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

/// Space defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "sill/settings.toml", domain = "spaces", page = Page::Spaces)]
pub struct SpacesSettings {
    #[settings(
        label = "Default grain",
        help = "The grain of a Space that has no look of its own (none by default; every preset takes it).",
        section = "Defaults",
        range = "0..=100",
        advanced
    )]
    pub default_grain: Count,
}

impl Default for SpacesSettings {
    fn default() -> Self {
        SpacesSettings {
            default_grain: Count(0),
        }
    }
}
