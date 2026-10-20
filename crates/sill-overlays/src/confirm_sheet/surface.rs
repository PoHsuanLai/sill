//! The confirmation sheet as a [`Surface`]: one per output, spawned hidden and mapped while the
//! intents service holds a question and the banners' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::confirm_sheet;
use sill_ui_kit::links::{BannerLink, Links};
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, PanelBlur, Place, Presence, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "confirm-panel";

/// The sheet that asks before intentd lets something that asks happen.
#[derive(Debug)]
pub enum ConfirmSheet {}

impl Surface for ConfirmSheet {
    const NAME: &'static str = "confirm";
    /// Only the output the link names shows the sheet.
    type Link = BannerLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::Main;
    const LAYER_ORDER: u8 = 90;
    const PRESENCE: Presence = Presence::SheetStage;

    fn link(links: &Links) -> BannerLink {
        links.banners.clone()
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
        confirm_sheet(output)
    }
}
