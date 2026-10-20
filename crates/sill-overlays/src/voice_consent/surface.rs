//! The consent sheet as a [`Surface`]: one per output, spawned hidden and mapped while the voice
//! service asks and the banners' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::voice_consent;
use sill_ui_kit::links::BannerLink;
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, PanelBlur, Place, Start, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "voice-consent-panel";

/// The sheet that asks before the microphone may open.
#[derive(Debug)]
pub enum VoiceConsentSheet {}

impl Surface for VoiceConsentSheet {
    const NAME: &'static str = "voice-consent";
    /// Only the output the link names shows the sheet.
    type Link = BannerLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::Main;

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
        voice_consent(output)
    }
}
