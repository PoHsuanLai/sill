//! The wallpaper as a [`Surface`]: one per output, shown at once.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::wallpaper;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The desktop background: the prepared image with a light/dark cross-fade.
#[derive(Debug)]
pub enum Wallpaper {}

impl Surface for Wallpaper {
    const NAME: &'static str = "wallpaper";
    type Link = ();
    type Slot = ();

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output)
    }

    fn start() -> Start {
        Start::Shown
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        wallpaper(output)
    }
}
