//! The wallpaper's render: an `Ds` root over two cross-fading image layers.

use dioxus::prelude::*;
use ds::{BlurState, Ds, Material, RootExtent};
use shell_host::OutputInfo;
use sill_model::ids::OutputName;

use super::io::use_crossfade;
use super::model::{file_url, image_for};
use crate::hooks::{use_appearance, use_service};
use sill_services::WallpaperService;

/// The wallpaper's own stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const WALLPAPER_CSS: &str = include_str!("../../style/wallpaper.css");

/// The wallpaper on `output`: an `Ds` root over two cross-fading image layers showing the
/// cached file for the resolved scheme (`ds::resolve`, the same call `Ds` makes internally, so
/// the two never disagree about which scheme is current).
pub(super) fn wallpaper(output: OutputInfo) -> Element {
    let environment = use_appearance()();
    let wallpapers = use_service::<WallpaperService>()();
    let on = output.name.clone().map(OutputName);
    let appearance = environment.settings.appearance.appearance();
    let resolved = ds::resolve(
        appearance,
        ds::SpaceLook::default().theme,
        environment.system,
    );
    let url = image_for(&wallpapers, on.as_ref()).map(|image| {
        let path = match resolved.scheme {
            ds::Scheme::Light => &image.light,
            ds::Scheme::Dark => &image.dark,
        };
        file_url(path)
    });
    let layers = use_crossfade(url);
    rsx! {
        Ds {
            appearance,
            system: environment.system,
            material: Material::Widget,
            extent: RootExtent::Viewport,
            blur: BlurState::Unavailable,
            style { {WALLPAPER_CSS} }
            div { class: "sill-wallpaper",
                for (slot , back , image) in layers {
                    div {
                        key: "{slot}",
                        class: "sill-wallpaper-layer",
                        "data-layer": back,
                        if let Some(url) = image {
                            img { src: "{url}", alt: "" }
                        }
                    }
                }
            }
        }
    }
}
