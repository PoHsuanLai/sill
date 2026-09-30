//! Edit Widgets as a [`Surface`]: one per output, spawned hidden and mapped by the link.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::edit_widgets;
use sill_ui_kit::links::EditWidgetsLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The widget gallery, a sheet at the bottom of the output.
#[derive(Debug)]
pub enum EditWidgets {}

impl Surface for EditWidgets {
    const NAME: &'static str = "edit-widgets";
    type Link = EditWidgetsLink;
    type Slot = ();

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output)
    }

    fn start() -> Start {
        Start::Hidden
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        edit_widgets(output)
    }
}
