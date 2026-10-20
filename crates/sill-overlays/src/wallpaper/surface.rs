//! The wallpaper as a [`Surface`]: one per output, shown at once.

use dioxus::prelude::Element;
use shell_host::{HostRoot, LayerConfig, OutputInfo};
use sill_model::desktop::ShellRole;
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::wallpaper;
use sill_ui_kit::links::Links;
use sill_ui_kit::surface::{Place, Presence, Surface};

/// The desktop background: the prepared image with a light/dark cross-fade.
#[derive(Debug)]
pub enum Wallpaper {}

impl Surface for Wallpaper {
    const NAME: &'static str = "wallpaper";
    type Link = ();
    type Slot = ();
    const ROLE: Option<ShellRole> = Some(ShellRole::Wallpaper);
    const LAYER_ORDER: u8 = 30;
    const PRESENCE: Presence = Presence::Always;

    fn link(_links: &Links) {}

    fn needs(root: HostRoot, links: &Links) -> HostRoot {
        root.context(links.actions.clone())
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        wallpaper(output)
    }
}
