//! The OSD as a [`Surface`]: one per output, spawned hidden and mapped by its own machine.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::osd;
use sill_ui_kit::links::OsdLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The indicator that shows a new volume or brightness level.
#[derive(Debug)]
pub enum Osd {}

impl Surface for Osd {
    const NAME: &'static str = "osd";
    type Link = OsdLink;
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
        osd(output)
    }
}
