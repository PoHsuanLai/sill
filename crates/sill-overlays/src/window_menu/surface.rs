//! The window menu as a [`Surface`]: one per output, mapped at once, drawing nothing; it opens
//! the menu's popup when the link asks for a menu on its output.

use dioxus::prelude::Element;
use shell_host::{LayerConfig, OutputInfo};
use sill_settings::ShellFile;

use super::layer::layer;
use super::view::window_menu;
use sill_ui_kit::links::WindowMenuLink;
use sill_ui_kit::surface::{Place, Start, Surface};

/// The transparent parent of a window's menu popup.
#[derive(Debug)]
pub enum WindowMenu {}

impl Surface for WindowMenu {
    const NAME: &'static str = "window-menu";
    type Link = WindowMenuLink;
    type Slot = ();

    fn slots(_place: &Place) -> Vec<()> {
        vec![()]
    }

    fn layer(output: &OutputInfo, _settings: &ShellFile, (): ()) -> LayerConfig {
        layer(output)
    }

    fn start() -> Start {
        Start::Shown
    }

    fn root(output: OutputInfo, (): ()) -> Element {
        window_menu(output)
    }
}
