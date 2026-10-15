//! The panel as a [`Surface`]: one per output, spawned hidden and mapped while the link holds an
//! opening on its output.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::force_quit;
use sill_ui_kit::links::ForceQuitLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The Force Quit Applications panel.
#[derive(Debug)]
pub enum ForceQuit {}

impl Surface for ForceQuit {
    const NAME: &'static str = "force-quit";
    type Link = ForceQuitLink;
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
        force_quit(output)
    }
}
