//! The eyedropper as a [`Surface`]: one per output, spawned hidden and mapped by the link's
//! pictures.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::color_sampler;
use sill_ui_kit::links::EyedropperLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The surface that lets the person point at a colour on the screen.
#[derive(Debug)]
pub enum ColorSampler {}

impl Surface for ColorSampler {
    const NAME: &'static str = "eyedropper";
    type Link = EyedropperLink;
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
        color_sampler(output)
    }
}
