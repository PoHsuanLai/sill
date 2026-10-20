//! The question as a [`Surface`]: one per output, spawned hidden and mapped while the link holds
//! an opening on its output.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::empty_trash;
use sill_ui_kit::links::{EmptyTrashLink, Links};
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, PanelBlur, Place, Presence, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "empty-trash-panel";

/// The layer: over the whole output, the pointer everywhere on it.
pub(super) const PRESET: LayerPreset = LayerPreset::FullOutputSheet {
    blur: Some(PanelBlur::sheet(PANEL_ID)),
};

/// The question whether to erase the Trash for good.
#[derive(Debug)]
pub enum EmptyTrashSheet {}

impl Surface for EmptyTrashSheet {
    const NAME: &'static str = "empty-trash";
    type Link = EmptyTrashLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::FocusedWindow;
    const LAYER_ORDER: u8 = 150;
    const PRESENCE: Presence = Presence::SheetStage;

    fn link(links: &Links) -> EmptyTrashLink {
        links.empty_trash.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        PRESET.config(Self::NAME, output)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        empty_trash(output)
    }
}
