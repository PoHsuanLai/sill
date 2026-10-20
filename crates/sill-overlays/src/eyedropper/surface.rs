//! The eyedropper as a [`Surface`]: one per output, spawned hidden and mapped by the link's
//! pictures.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::color_sampler;
use sill_ui_kit::links::{EyedropperLink, Links};
use sill_ui_kit::surface::{LayerPreset, Place, Presence, Surface};

/// The surface that lets the person point at a colour on the screen.
#[derive(Debug)]
pub enum ColorSampler {}

impl Surface for ColorSampler {
    const NAME: &'static str = "eyedropper";
    type Link = EyedropperLink;
    type Slot = ();
    const LAYER_ORDER: u8 = 150;
    const PRESENCE: Presence = Presence::FollowsPanel;

    fn link(links: &Links) -> EyedropperLink {
        links.eyedropper.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        LayerPreset::FullOutputSheet { blur: None }.config(Self::NAME, output)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        color_sampler(output)
    }
}
