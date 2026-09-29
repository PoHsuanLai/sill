//! The daemon's notifications half: the banners' output. The banner column and the notification
//! center are spawned like any `Surface` (`sill_ui_kit::{Banners, NotificationCenter}`).

use sill_ui_kit::Banners;
use sill_ui_kit::links::BannerOutput;
use sill_ui_kit::surface::Surface;

use crate::outputs::Outputs;

/// The output the banners show on: the first with a banner column (the main display's
/// stand-in, `banners/placement.rs`).
pub fn banner_output(outputs: &Outputs) -> BannerOutput {
    outputs
        .outputs
        .iter()
        .find(|(_, s)| s.spawned.iter().any(|kind| kind.name == Banners::NAME))
        .map_or(BannerOutput::Nowhere, |(info, _)| BannerOutput::On(info.id))
}

#[cfg(test)]
mod tests;
