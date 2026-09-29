//! The banner column as a [`Surface`]: one per output, spawned hidden, mapped while a card is up.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::banners;
use crate::links::BannerLink;
use crate::surface::{Place, Start, Surface};

/// The notification banners, stacked down the main display's right edge.
#[derive(Debug)]
pub enum Banners {}

impl Surface for Banners {
    const NAME: &'static str = "banners";
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
        banners(output)
    }
}
