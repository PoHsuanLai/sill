//! The screenshot thumbnail as a [`Surface`]: one per output, spawned hidden, shown on the
//! output the banners' link names.

use dioxus::prelude::Element;
use shell_host::{KeyboardFocus, LayerConfig, OutputId, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::screenshot_thumb;
use sill_ui_kit::links::{BannerLink, Links, Transient};
use sill_ui_kit::surface::{FocusLoss, Place, Presence, Surface};

/// The compositor taking the keyboard from the thumbnail's surface is told on the link, which says which
/// transient lost it.
fn keyboard_lost(link: &BannerLink, _output: OutputId, focus: KeyboardFocus) {
    if focus == KeyboardFocus::Lost {
        link.keyboard_lost(Transient::Thumbnail);
    }
}

/// The card that rises at the bottom right after a screenshot.
#[derive(Debug)]
pub enum ScreenshotThumb {}

impl Surface for ScreenshotThumb {
    const NAME: &'static str = "screenshot-thumb";
    type Link = BannerLink;
    type Slot = ();
    const KEYBOARD: FocusLoss<BannerLink> = FocusLoss::CloseLink(keyboard_lost);
    const LAYER_ORDER: u8 = 200;
    const PRESENCE: Presence = Presence::SelfMapped;

    fn link(links: &Links) -> BannerLink {
        links.banners.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output, settings)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        screenshot_thumb(output)
    }
}
