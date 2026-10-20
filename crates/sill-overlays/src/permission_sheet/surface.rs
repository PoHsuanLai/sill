//! The permission sheet as a [`Surface`]: one per output, spawned hidden and mapped while the
//! capture service holds a question and the banners' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::permission_sheet;
use sill_ui_kit::links::{BannerLink, Links};
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, PanelBlur, Place, Presence, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "permission-panel";

/// The layer: over the whole output, but the pointer only on the panel, so a click beside the
/// sheet reaches the window under it (the sheet draws no scrim and closes on no outside press).
pub(super) const PRESET: LayerPreset = LayerPreset::PanelOnly {
    panel: PanelBlur::sheet(PANEL_ID),
};

/// The sheet that asks whether an app may do something.
#[derive(Debug)]
pub enum PermissionSheet {}

impl Surface for PermissionSheet {
    const NAME: &'static str = "permission";
    /// Only the output the link names shows the sheet.
    type Link = BannerLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::Main;
    const LAYER_ORDER: u8 = 130;
    const PRESENCE: Presence = Presence::SheetStage;

    fn link(links: &Links) -> BannerLink {
        links.banners.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        PRESET.config(Self::NAME, output)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        permission_sheet(output)
    }
}
