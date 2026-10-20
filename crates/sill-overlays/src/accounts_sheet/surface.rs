//! The accounts sheet as a [`Surface`]: one per output, spawned hidden and mapped while accountd
//! has a sheet open and the banners' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::accounts_sheet;
use sill_ui_kit::links::BannerLink;
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, Place, Start, Surface};

/// The sheet accountd draws its consent question and add-account steps on.
#[derive(Debug)]
pub enum AccountsSheet {}

impl Surface for AccountsSheet {
    const NAME: &'static str = "accounts-sheet";
    /// Only the output the link names shows the sheet.
    type Link = BannerLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::Main;

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        LayerPreset::FullOutputSheet { blur: None }.config(Self::NAME, output)
    }

    fn start() -> Start {
        Start::Hidden
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        accounts_sheet(output)
    }
}
