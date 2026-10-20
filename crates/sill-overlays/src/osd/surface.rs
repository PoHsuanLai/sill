//! The OSD as a [`Surface`]: one per output, spawned hidden and mapped by its own machine.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_model::desktop::ShellRole;
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::osd;
use sill_ui_kit::links::{Links, OsdLink};
use sill_ui_kit::surface::{Place, Presence, Surface};

/// The indicator that shows a new volume or brightness level.
#[derive(Debug)]
pub enum Osd {}

impl Surface for Osd {
    const NAME: &'static str = "osd";
    type Link = OsdLink;
    type Slot = ();
    const ROLE: Option<ShellRole> = Some(ShellRole::Osd);
    const LAYER_ORDER: u8 = 240;
    const PRESENCE: Presence = Presence::SelfMapped;

    fn link(links: &Links) -> OsdLink {
        links.osd.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output, settings)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        osd(output)
    }
}
