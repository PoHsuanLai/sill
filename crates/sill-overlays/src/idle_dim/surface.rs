//! The idle dim as a [`Surface`]: one per output, spawned warm and hidden; the daemon maps it
//! only while the idle service dims.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::idle_dim;
use sill_ui_kit::links::Links;
use sill_ui_kit::surface::{Place, Presence, Surface};

/// The black scrim over every output while the idle service dims.
#[derive(Debug)]
pub enum IdleDim {}

impl Surface for IdleDim {
    const NAME: &'static str = "idle-dim";
    type Link = ();
    type Slot = ();
    const LAYER_ORDER: u8 = 250;
    const PRESENCE: Presence = Presence::Daemon;

    fn link(_links: &Links) {}

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        idle_dim(output)
    }
}
