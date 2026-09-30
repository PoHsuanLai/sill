//! The wallpaper as a [`Surface`]: one per output while the shell draws the background, shown
//! at once.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::{ShellFile, WallpaperDrawer};

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

    fn slots(place: &Place) -> Vec<()> {
        match place.settings.spaces.wallpaper_drawer {
            WallpaperDrawer::Shell => vec![()],
            _ => Vec::new(),
        }
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
