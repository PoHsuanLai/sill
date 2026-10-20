//! The picker as a [`Surface`]: one per output, spawned hidden and mapped while the capture
//! service holds a request and the banners' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::capture_sheet;
use sill_ui_kit::links::BannerLink;
use sill_ui_kit::surface::{LayerPreset, PanelBlur, Place, Start, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "capture-panel";

/// The sheet that asks what an app may see of the screen.
#[derive(Debug)]
pub enum CaptureSheet {}

impl Surface for CaptureSheet {
    const NAME: &'static str = "capture";
    /// Only the output the link names shows the sheet.
    type Link = BannerLink;
    type Slot = ();

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        LayerPreset::FullOutputSheet {
            blur: Some(PanelBlur::sheet(PANEL_ID)),
        }
        .config(Self::NAME, output)
    }

    fn start() -> Start {
        Start::Hidden
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        capture_sheet(output)
    }
}
