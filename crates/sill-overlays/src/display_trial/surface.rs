//! The display trial as a [`Surface`]: one per output, spawned hidden and mapped while the link
//! holds a trial and the banners' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::display_trial;
use sill_ui_kit::links::DisplayTrialLink;
use sill_ui_kit::surface::{LayerPreset, PanelBlur, Place, Start, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "display-trial-panel";

/// The question whether to keep a changed display setup.
#[derive(Debug)]
pub enum DisplayTrialSheet {}

impl Surface for DisplayTrialSheet {
    const NAME: &'static str = "display-trial";
    /// The link also names the output that shows the sheet.
    type Link = DisplayTrialLink;
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
        display_trial(output)
    }
}
