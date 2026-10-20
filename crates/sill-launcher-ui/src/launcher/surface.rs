//! The launcher's panel as a [`Surface`]: one per output, spawned hidden and mapped by the
//! shared [`LauncherLink`].

use dioxus::prelude::Element;
use shell_host::{HostRoot, LayerConfig, OutputInfo};
use sill_model::desktop::ShellRole;
use sill_settings::ShellFile;

use super::layer::layer;
use super::panel::launcher;
use sill_ui_kit::links::{LauncherLink, Links};
use sill_ui_kit::surface::{ClosesOnFocusLoss, FocusLoss, OutputPolicy, Place, Presence, Surface};

/// The centred search panel: quire's `CommandPalette` over the runner's results.
#[derive(Debug)]
pub enum Launcher {}

impl Surface for Launcher {
    const NAME: &'static str = "launcher";
    type Link = LauncherLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::FocusedWindow;
    const KEYBOARD: FocusLoss<LauncherLink> = FocusLoss::CloseLink(LauncherLink::keyboard);
    const ROLE: Option<ShellRole> = Some(ShellRole::Launcher);
    const LAYER_ORDER: u8 = 60;
    const PRESENCE: Presence = Presence::FollowsPanel;

    fn link(links: &Links) -> LauncherLink {
        links.launcher.clone()
    }

    fn needs(root: HostRoot, links: &Links) -> HostRoot {
        root.context(links.actions.clone())
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output, settings)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        launcher(output)
    }
}
