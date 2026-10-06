//! The consent sheet as a [`Surface`]: one per output, spawned hidden and mapped while the voice
//! service asks and the banners' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::voice_consent;
use sill_ui_kit::links::BannerLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The sheet that asks before the microphone may open.
#[derive(Debug)]
pub enum VoiceConsentSheet {}

impl Surface for VoiceConsentSheet {
    const NAME: &'static str = "voice-consent";
    /// Only the output the link names shows the sheet.
    type Link = BannerLink;
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
        voice_consent(output)
    }
}
