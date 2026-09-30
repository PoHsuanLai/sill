//! The notification center as a [`Surface`]: one per output, spawned hidden and kept warm.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::notification_center;
use sill_ui_kit::links::CenterLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The history grouped by app, in a column at the output's right edge.
#[derive(Debug)]
pub enum NotificationCenter {}

impl Surface for NotificationCenter {
    const NAME: &'static str = "notification-center";
    type Link = CenterLink;
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
        notification_center(output)
    }
}
