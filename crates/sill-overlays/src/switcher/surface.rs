//! The app switcher as a [`Surface`]: one per output, spawned hidden and mapped from the chord
//! until it closes.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::switcher;
use sill_ui_kit::links::SwitcherLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// Cmd+Tab over applications, not windows.
#[derive(Debug)]
pub enum Switcher {}

impl Surface for Switcher {
    const NAME: &'static str = "switcher";
    type Link = SwitcherLink;
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
        switcher(output)
    }
}
