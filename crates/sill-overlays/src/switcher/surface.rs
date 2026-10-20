//! The app switcher as a [`Surface`]: one per output, spawned hidden and mapped from the chord
//! until it closes.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::switcher;
use sill_ui_kit::links::SwitcherLink;
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, PanelBlur, Place, Start, Surface};

/// The panel's element id: the surface's input and blur region.
pub(super) const PANEL_ID: &str = "switcher-panel";

/// The panel's corner: `Material::Osd`'s radius (design/13 §13.3.5), not a sheet's.
const PANEL_RADIUS: i32 = 18;

/// Cmd+Tab over applications, not windows.
#[derive(Debug)]
pub enum Switcher {}

impl Surface for Switcher {
    const NAME: &'static str = "switcher";
    type Link = SwitcherLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::FocusedWindow;

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        LayerPreset::PanelOnly {
            panel: PanelBlur {
                id: PANEL_ID,
                radius: PANEL_RADIUS,
            },
        }
        .config(Self::NAME, output)
    }

    fn start() -> Start {
        Start::Hidden
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        switcher(output)
    }
}
