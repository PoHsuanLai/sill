//! The display trial as a [`Surface`]: one per output, spawned hidden and mapped while the link
//! holds a trial and the sheets' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::display_trial;
use sill_ui_kit::links::{DisplayTrialLink, Links};
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, PanelBlur, Place, Presence, Surface};

/// The panel's element id: the surface's blur region.
pub(super) const PANEL_ID: &str = "display-trial-panel";

/// The layer: over the whole output, the pointer everywhere on it.
pub(super) const PRESET: LayerPreset = LayerPreset::FullOutputSheet {
    blur: Some(PanelBlur::sheet(PANEL_ID)),
};

/// The question whether to keep a changed display setup.
#[derive(Debug)]
pub enum DisplayTrialSheet {}

impl Surface for DisplayTrialSheet {
    const NAME: &'static str = "display-trial";
    /// The sheets' link names the output that shows the sheet.
    type Link = DisplayTrialLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::FocusedWindow;
    const LAYER_ORDER: u8 = 140;
    const PRESENCE: Presence = Presence::SheetStage;

    fn link(links: &Links) -> DisplayTrialLink {
        links.display_trial.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        PRESET.config(Self::NAME, output)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        display_trial(output)
    }
}
