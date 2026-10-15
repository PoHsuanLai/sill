//! The Split View picker as a [`Surface`]: one per output, spawned hidden and mapped by the
//! link's offer.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::split_picker;
use sill_ui_kit::links::SplitPickerLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The choice of the second window of a Split View.
#[derive(Debug)]
pub enum SplitPicker {}

impl Surface for SplitPicker {
    const NAME: &'static str = "split-picker";
    type Link = SplitPickerLink;
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
        split_picker(output)
    }
}
