//! The power menu as a [`Surface`]: one per output, spawned hidden and mapped by its own link.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::power_menu;
use sill_ui_kit::links::{Links, PowerMenuLink};
use sill_ui_kit::surface::{
    ClosesOnFocusLoss, FocusLoss, LayerPreset, OutputPolicy, PanelBlur, Place, Presence, Surface,
};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "power-menu-panel";

/// The layer: over the whole output, the pointer everywhere on it.
pub(super) const PRESET: LayerPreset = LayerPreset::FullOutputSheet {
    blur: Some(PanelBlur::sheet(PANEL_ID)),
};

/// The scrim and sheet that ask before Log Out, Restart, Shut Down or Suspend.
#[derive(Debug)]
pub enum PowerMenu {}

impl Surface for PowerMenu {
    const NAME: &'static str = "power-menu";
    type Link = PowerMenuLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::FocusedWindow;
    const KEYBOARD: FocusLoss<PowerMenuLink> = FocusLoss::CloseLink(PowerMenuLink::keyboard);
    const LAYER_ORDER: u8 = 70;
    const PRESENCE: Presence = Presence::FollowsPanel;

    fn link(links: &Links) -> PowerMenuLink {
        links.power_menu.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        PRESET.config(Self::NAME, output)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        power_menu(output)
    }
}
