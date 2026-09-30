//! The launcher's panel as a [`Surface`]: one per output, spawned hidden and mapped by the
//! shared [`LauncherLink`].

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::panel::launcher;
use sill_ui_kit::links::LauncherLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The centred search panel: quire's `CommandPalette` over the runner's results.
#[derive(Debug)]
pub enum Launcher {}

impl Surface for Launcher {
    const NAME: &'static str = "launcher";
    type Link = LauncherLink;
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
        launcher(output)
    }
}
