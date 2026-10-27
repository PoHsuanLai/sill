//! The bar as a [`Surface`]: one per output, shown at once.

use dioxus::prelude::Element;
use shell_host::{HostRoot, LayerConfig, OutputInfo};
use sill_model::desktop::ShellRole;
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::bar;
use sill_ui_kit::links::{ControlCenterLink, Links};
use sill_ui_kit::surface::{OutputPolicy, Place, Presence, Surface};

/// The menu bar along the top of an output, with its menus and status items.
#[derive(Debug)]
pub enum Bar {}

impl Surface for Bar {
    const NAME: &'static str = "bar";
    type Link = ControlCenterLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::FocusedWindow;
    const ROLE: Option<ShellRole> = Some(ShellRole::Bar);
    const LAYER_ORDER: u8 = 20;
    const PRESENCE: Presence = Presence::Always;

    fn link(links: &Links) -> ControlCenterLink {
        links.control_center.clone()
    }

    fn needs(root: HostRoot, links: &Links) -> HostRoot {
        root.context(links.focus.clone())
            .context(links.actions.clone())
            .context(links.notification_center.clone())
            .context(links.edges.clone())
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output, settings.bar.autohide)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        bar(output)
    }
}
