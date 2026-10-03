//! The confirmation sheet as a [`Surface`]: one per output, spawned hidden and mapped while the
//! intents service holds a question and the banners' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::confirm_sheet;
use sill_ui_kit::links::BannerLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The sheet that asks before intentd lets something that asks happen.
#[derive(Debug)]
pub enum ConfirmSheet {}

impl Surface for ConfirmSheet {
    const NAME: &'static str = "confirm";
    /// Only the output the link names shows the sheet.
    type Link = BannerLink;
    type Slot = ();

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output, settings)
    }

    fn start() -> Start {
        Start::Hidden
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        confirm_sheet(output)
    }
}
