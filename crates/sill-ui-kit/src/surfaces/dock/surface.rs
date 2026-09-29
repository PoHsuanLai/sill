//! The dock as a [`Surface`]: one per output, shown at once.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::dock;
use crate::surface::{Place, Start, Surface};

/// The pill of app tiles along the bottom edge of an output.
#[derive(Debug)]
pub enum Dock {}

impl Surface for Dock {
    const NAME: &'static str = "dock";
    type Link = ();
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
        dock(output)
    }
}
