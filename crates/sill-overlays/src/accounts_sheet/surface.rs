//! The accounts sheet as a [`Surface`]: one per output, spawned hidden and mapped while accountd
//! has a sheet open and the banners' output names this one.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::accounts_sheet;
use sill_ui_kit::links::{BannerLink, Links};
use sill_ui_kit::surface::{LayerPreset, OutputPolicy, Place, Presence, Surface};

/// The layer: over the whole output, the pointer everywhere on it while the sheet is up. The
/// surface unmaps at once when the step goes (no exit is played), so there is no stage to pass
/// the pointer through.
pub(super) const PRESET: LayerPreset = LayerPreset::FullOutputSheet { blur: None };

/// The sheet accountd draws its consent question and add-account steps on.
#[derive(Debug)]
pub enum AccountsSheet {}

impl Surface for AccountsSheet {
    const NAME: &'static str = "accounts-sheet";
    /// Only the output the link names shows the sheet.
    type Link = BannerLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::Main;
    const LAYER_ORDER: u8 = 110;
    const PRESENCE: Presence = Presence::SelfMapped;

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
        accounts_sheet(output)
    }
}
