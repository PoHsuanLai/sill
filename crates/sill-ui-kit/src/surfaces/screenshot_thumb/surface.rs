//! The screenshot thumbnail as a [`Surface`]: one per output, spawned hidden, shown on the
//! output the banners' link names.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::screenshot_thumb;
use crate::links::BannerLink;
use crate::surface::{Place, Start, Surface};

/// The card that rises at the bottom right after a screenshot.
#[derive(Debug)]
pub enum ScreenshotThumb {}

impl Surface for ScreenshotThumb {
    const NAME: &'static str = "screenshot-thumb";
    type Link = BannerLink;
    type Slot = ();

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output, settings)
    }

    fn start() -> Start {
        Start::Hidden
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        screenshot_thumb(output)
    }
}
