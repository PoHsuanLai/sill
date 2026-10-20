//! The consent sheet as a [`Surface`]: one per output, spawned hidden and mapped while the voice
//! service asks and the banners' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::voice_consent;
use sill_ui_kit::links::{BannerLink, Links};
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, PanelBlur, Place, Presence, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "voice-consent-panel";

/// The layer: over the whole output, the pointer everywhere on it.
pub(super) const PRESET: LayerPreset = LayerPreset::FullOutputSheet {
    blur: Some(PanelBlur::sheet(PANEL_ID)),
};

/// The sheet that asks before the microphone may open.
#[derive(Debug)]
pub enum VoiceConsentSheet {}

impl Surface for VoiceConsentSheet {
    const NAME: &'static str = "voice-consent";
    /// Only the output the link names shows the sheet.
    type Link = BannerLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::Main;
    const LAYER_ORDER: u8 = 100;
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
        voice_consent(output)
    }
}
