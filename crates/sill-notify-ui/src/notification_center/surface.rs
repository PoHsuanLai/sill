//! The notification center as a [`Surface`]: one per output, spawned hidden and kept warm.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_model::desktop::ShellRole;
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::notification_center;
use sill_ui_kit::links::{CenterLink, Links};
use sill_ui_kit::surface::{ClosesOnFocusLoss, FocusLoss, OutputPolicy, Place, Presence, Surface};

/// The history grouped by app, in a column at the output's right edge.
#[derive(Debug)]
pub enum NotificationCenter {}

impl Surface for NotificationCenter {
    const NAME: &'static str = "notification-center";
    type Link = CenterLink;
    type Slot = ();
    const OUTPUTS: OutputPolicy = OutputPolicy::FocusedWindow;
    const KEYBOARD: FocusLoss<CenterLink> = FocusLoss::CloseLink(CenterLink::keyboard);
    const ROLE: Option<ShellRole> = Some(ShellRole::Notifications);
    const LAYER_ORDER: u8 = 190;
    const PRESENCE: Presence = Presence::SelfMapped;

    fn link(links: &Links) -> CenterLink {
        links.notification_center.clone()
    }

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output, settings)
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        notification_center(output)
    }
}
