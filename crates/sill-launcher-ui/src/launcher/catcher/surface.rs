//! The launcher's catcher as a [`Surface`]: one per output, spawned hidden and mapped with the
//! panel.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::catcher;
use sill_ui_kit::links::{LauncherLink, Links};
use sill_ui_kit::surface::{OutputPolicy, Place, Presence, Surface};

/// The empty, transparent, full-output surface whose click closes the launcher.
#[derive(Debug)]
pub enum LauncherCatcher {}

impl Surface for LauncherCatcher {
    const NAME: &'static str = "launcher-catcher";
    type Link = LauncherLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::FocusedWindow;
    const LAYER_ORDER: u8 = 50;
    const PRESENCE: Presence = Presence::FollowsPanel;

    fn link(links: &Links) -> LauncherLink {
        links.launcher.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        catcher(output)
    }
}
