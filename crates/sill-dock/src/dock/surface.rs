//! The dock as a [`Surface`]: one per output, shown at once.

use dioxus::prelude::Element;
use shell_host::{HostRoot, LayerConfig, OutputInfo};
use sill_model::desktop::ShellRole;
use sill_settings::ShellFile;

use super::layer::layer;
use super::tokens::DockTokens;
use super::view::dock;
use sill_ui_kit::links::Links;
use sill_ui_kit::surface::{Place, Presence, Surface};

/// The pill of app tiles along the bottom edge of an output.
#[derive(Debug)]
pub enum Dock {}

impl Surface for Dock {
    const NAME: &'static str = "dock";
    type Link = ();
    type Slot = ();
    const ROLE: Option<ShellRole> = Some(ShellRole::Dock);
    const LAYER_ORDER: u8 = 40;
    const PRESENCE: Presence = Presence::Always;

    fn link(_links: &Links) {}

    fn needs(root: HostRoot, links: &Links) -> HostRoot {
        root.context(links.actions.clone())
            .context(links.edges.clone())
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, (): ()) -> LayerConfig {
        // The shipped tokens: the surface has no document yet; `dock` follows its layer with the
        // resolved ones once it is mounted.
        layer(output, settings, &DockTokens::default())
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        dock(output)
    }
}
