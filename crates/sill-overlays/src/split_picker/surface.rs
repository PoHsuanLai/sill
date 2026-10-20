//! The Split View picker as a [`Surface`]: one per output, spawned hidden and mapped by the
//! link's offer.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::view::split_picker;
use sill_ui_kit::links::{Links, SplitPickerLink};
use sill_ui_kit::surface::{ClosesOnFocusLoss, FocusLoss, LayerPreset, Place, Presence, Surface};

/// The choice of the second window of a Split View.
#[derive(Debug)]
pub enum SplitPicker {}

impl Surface for SplitPicker {
    const NAME: &'static str = "split-picker";
    type Link = SplitPickerLink;
    type Slot = ();
    const KEYBOARD: FocusLoss<SplitPickerLink> = FocusLoss::CloseLink(SplitPickerLink::keyboard);
    const LAYER_ORDER: u8 = 230;
    const PRESENCE: Presence = Presence::FollowsPanel;

    fn link(links: &Links) -> SplitPickerLink {
        links.split_picker.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        LayerPreset::FullOutputSheet { blur: None }.config(Self::NAME, output)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        split_picker(output)
    }
}
