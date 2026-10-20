//! The power menu as a [`Surface`]: one per output, spawned hidden and mapped by its own link.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::power_menu;
use sill_ui_kit::links::PowerMenuLink;
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, PanelBlur, Place, Start, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "power-menu-panel";

/// The scrim and sheet that ask before Log Out, Restart, Shut Down or Suspend.
#[derive(Debug)]
pub enum PowerMenu {}

impl Surface for PowerMenu {
    const NAME: &'static str = "power-menu";
    type Link = PowerMenuLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::FocusedWindow;

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        LayerPreset::FullOutputSheet {
            blur: Some(PanelBlur::sheet(PANEL_ID)),
        }
        .config(Self::NAME, output)
    }

    fn start() -> Start {
        Start::Hidden
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        power_menu(output)
    }
}
