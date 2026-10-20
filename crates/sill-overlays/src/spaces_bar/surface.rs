//! The Spaces bar as a [`Surface`]: one per output, spawned hidden and mapped while Mission
//! Control is open.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::spaces_bar;
use sill_ui_kit::links::{Links, SpacesBarLink};
use sill_ui_kit::surface::{Place, Presence, Surface};

/// Mission Control's Spaces bar.
#[derive(Debug)]
pub enum SpacesBar {}

impl Surface for SpacesBar {
    const NAME: &'static str = "spaces-bar";
    type Link = SpacesBarLink;
    type Slot = ();
    const LAYER_ORDER: u8 = 10;
    const PRESENCE: Presence = Presence::SelfMapped;

    fn link(links: &Links) -> SpacesBarLink {
        links.spaces_bar.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output, settings)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        spaces_bar(output)
    }
}
