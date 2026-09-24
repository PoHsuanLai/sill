//! The wallpaper (design/20 §1.4): one per output, Background layer, no input, no blur, the
//! prepared image with a light/dark cross-fade over `--t-big`, rendered only on change.

#![allow(unused_variables, dead_code)] // Freeze stubs

pub mod layer;

pub use layer::wallpaper_layer;

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The wallpaper on `output`: one `img` of the cached file for the resolved scheme.
#[component]
pub fn Wallpaper(output: OutputInfo) -> Element {
    todo!("M2 wallpaper")
}
