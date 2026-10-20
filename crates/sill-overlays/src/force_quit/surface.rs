//! The panel as a [`Surface`]: one per output, spawned hidden and mapped while the link holds an
//! opening on its output.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::force_quit;
use sill_ui_kit::links::{ForceQuitLink, Links};
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, PanelBlur, Place, Presence, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "force-quit-panel";

/// The Force Quit Applications panel.
#[derive(Debug)]
pub enum ForceQuit {}

impl Surface for ForceQuit {
    const NAME: &'static str = "force-quit";
    type Link = ForceQuitLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::FocusedWindow;
    const LAYER_ORDER: u8 = 160;
    const PRESENCE: Presence = Presence::SheetStage;

    fn link(links: &Links) -> ForceQuitLink {
        links.force_quit.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        LayerPreset::FullOutputSheet {
            blur: Some(PanelBlur::sheet(PANEL_ID)),
        }
        .config(Self::NAME, output)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        force_quit(output)
    }
}
