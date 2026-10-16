//! The permission sheet as a [`Surface`]: one per output, spawned hidden and mapped while the
//! capture service holds a question and the banners' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::permission_sheet;
use sill_ui_kit::links::BannerLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The sheet that asks whether an app may do something.
#[derive(Debug)]
pub enum PermissionSheet {}

impl Surface for PermissionSheet {
    const NAME: &'static str = "permission";
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
        permission_sheet(output)
    }
}
