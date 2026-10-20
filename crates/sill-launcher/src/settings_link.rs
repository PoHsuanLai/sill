//! Where in Settings a result or a button points, as the text every entrance hands over.
//!
//! The typed form is `ds_settings::target::SettingsTarget` (`page:wallpaper`,
//! `key:dock.magnified_size_px`, `account:<id>`). This crate is pure and so cannot name that
//! crate, which brings the bus with it; it carries the text, built with `SettingsTarget`'s
//! `Display` where the link is made (the search provider, the bar, the notice service) and sent
//! unchanged to Settings.

use serde::{Deserialize, Serialize};

/// The text form of a `SettingsTarget`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SettingsLink(pub String);
