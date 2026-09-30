//! The polkit prompt as a [`Surface`]: one per output, spawned hidden and mapped while the
//! agent holds a request and the banners' output names it.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::polkit_prompt;
use sill_ui_kit::links::BannerLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The sheet that asks for a password when polkit wants an authentication.
#[derive(Debug)]
pub enum PolkitPrompt {}

impl Surface for PolkitPrompt {
    const NAME: &'static str = "polkit";
    /// Only the output the link names shows the prompt.
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
        polkit_prompt(output)
    }
}
