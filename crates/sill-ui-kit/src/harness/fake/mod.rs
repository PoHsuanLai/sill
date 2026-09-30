//! Fake services and the values they start at, for tests, demos and the surface harness.

pub mod dock;
mod feeds;
mod fixtures;

pub use feeds::{Feeds, services};
pub use fixtures::{
    battery, bluetooth, clock, kbd_backlight, media, network, output, output_named, power_profiles,
    toplevels, toplevels_on, volume, wifi, workspaces, workspaces_on,
};
