//! The bar as a [`Surface`]: one per output, shown at once.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::bar;
use crate::links::ControlCenterLink;
use crate::surface::{Place, Start, Surface};

/// The menu bar along the top of an output, with its menus and status items.
#[derive(Debug)]
pub enum Bar {}

impl Surface for Bar {
    const NAME: &'static str = "bar";
    type Link = ControlCenterLink;
    type Slot = ();

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output, settings)
    }

    fn start() -> Start {
        Start::Shown
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        bar(output)
    }
}
