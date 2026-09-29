//! The hot corners as a [`Surface`]: a slot per corner whose action is not `None`, shown at
//! once.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::model::active_corners;
use super::view::hot_corner;
use crate::links::{Corner, CornerLink};
use crate::surface::{Place, Start, Surface};

/// The transparent squares in the corners of an output that fire an action on a dwell.
#[derive(Debug)]
pub enum HotCorners {}

impl Surface for HotCorners {
    const NAME: &'static str = "hot-corner";
    type Link = CornerLink;
    type Slot = Corner;

    fn slots(place: &Place) -> Vec<Corner> {
        active_corners(&place.settings.hot_corners)
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, corner: Corner) -> LayerConfig {
        layer(output, settings, corner)
    }

    fn start() -> Start {
        Start::Shown
    }

    fn root(output: OutputInfo, corner: Corner) -> Element {
        hot_corner(output, corner)
    }
}
