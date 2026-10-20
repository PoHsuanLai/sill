//! The polkit prompt as a [`Surface`]: one per output, spawned hidden and mapped while the
//! agent holds a request and the banners' output names it.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::polkit_prompt;
use sill_ui_kit::links::{BannerLink, Links};
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, PanelBlur, Place, Presence, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "polkit-panel";

/// The layer: over the whole output, but the pointer only on the panel, so a click beside the
/// sheet reaches the window under it (the sheet draws no scrim and closes on no outside press).
pub(super) const PRESET: LayerPreset = LayerPreset::PanelOnly {
    panel: PanelBlur::sheet(PANEL_ID),
};

/// The sheet that asks for a password when polkit wants an authentication.
#[derive(Debug)]
pub enum PolkitPrompt {}

impl Surface for PolkitPrompt {
    const NAME: &'static str = "polkit";
    /// Only the output the link names shows the prompt.
    type Link = BannerLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::Main;
    const LAYER_ORDER: u8 = 80;
    const PRESENCE: Presence = Presence::SheetStage;

    fn link(links: &Links) -> BannerLink {
        links.banners.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        PRESET.config(Self::NAME, output)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        polkit_prompt(output)
    }
}
