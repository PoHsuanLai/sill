//! The polkit prompt as a [`Surface`]: one per output, spawned hidden and mapped while the
//! agent holds a request and the banners' output names it.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::polkit_prompt;
use sill_ui_kit::links::BannerLink;
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, PanelBlur, Place, Start, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "polkit-panel";

/// The sheet that asks for a password when polkit wants an authentication.
#[derive(Debug)]
pub enum PolkitPrompt {}

impl Surface for PolkitPrompt {
    const NAME: &'static str = "polkit";
    /// Only the output the link names shows the prompt.
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
        polkit_prompt(output)
    }
}
