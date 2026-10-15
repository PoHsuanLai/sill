//! The Spaces bar as a [`Surface`]: one per output, spawned hidden and mapped while Mission
//! Control is open.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::spaces_bar;
use sill_ui_kit::links::SpacesBarLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// Mission Control's Spaces bar.
#[derive(Debug)]
pub enum SpacesBar {}

impl Surface for SpacesBar {
    const NAME: &'static str = "spaces-bar";
    type Link = SpacesBarLink;
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
        spaces_bar(output)
    }
}
