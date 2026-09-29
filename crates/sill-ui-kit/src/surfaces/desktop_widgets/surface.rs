//! The desktop widget layer as a [`Surface`]: one per output while `widgets.desktop` is `On`,
//! closed when it turns `Off`, shown at once.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::{DesktopWidgets as Desktop, ShellFile};

use super::layer::layer;
use super::view::desktop;
use crate::links::PlacementFile;
use crate::surface::{Place, Start, Surface};

/// The widgets on the desktop, above the wallpaper and under every window.
#[derive(Debug)]
pub enum DesktopWidgets {}

impl Surface for DesktopWidgets {
    const NAME: &'static str = "desktop-widgets";
    type Link = PlacementFile;
    type Slot = ();

    fn slots(place: &Place) -> Vec<()> {
        match place.settings.widgets.desktop {
            Desktop::On => vec![()],
            Desktop::Off => Vec::new(),
        }
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output)
    }

    fn start() -> Start {
        Start::Shown
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        desktop(output)
    }
}
