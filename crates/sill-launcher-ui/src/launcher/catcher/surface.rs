//! The launcher's catcher as a [`Surface`]: one per output, spawned hidden and mapped with the
//! panel.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::catcher;
use sill_ui_kit::links::LauncherLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The empty, transparent, full-output surface whose click closes the launcher.
#[derive(Debug)]
pub enum LauncherCatcher {}

impl Surface for LauncherCatcher {
    const NAME: &'static str = "launcher-catcher";
    type Link = LauncherLink;
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
        catcher(output)
    }
}
