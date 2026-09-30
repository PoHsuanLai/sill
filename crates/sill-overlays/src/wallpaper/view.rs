//! The wallpaper's render: an `Ds` root over two cross-fading image layers.

use dioxus::prelude::*;
use ds::prelude::*;
use ds::style::appearance::blur::BlurState;
use shell_host::OutputInfo;
use sill_model::ids::OutputName;

use super::io::use_crossfade;
use super::model::{file_url, image_for};
use sill_services::WallpaperService;
use sill_ui_kit::hooks::{use_appearance, use_service};
use sill_ui_kit::root::use_root_style;

/// The wallpaper's own stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const WALLPAPER_CSS: &str = include_str!("style.css");

/// The wallpaper on `output`: an `Ds` root over two cross-fading image layers showing the
/// cached file for the resolved scheme (`ds::prelude::resolve`, the same call `Ds` makes internally, so
/// the two never disagree about which scheme is current).
pub(super) fn wallpaper(output: OutputInfo) -> Element {
    let environment = use_appearance()();
    let wallpapers = use_service::<WallpaperService>()();
    let on = output.name.clone().map(OutputName);
    let appearance = environment.settings.appearance.appearance();
    let resolved = resolve(appearance, SpaceLook::default().theme, environment.system);
    let url = image_for(&wallpapers, on.as_ref()).map(|image| {
        let path = match resolved.scheme {
            Scheme::Light => &image.light,
            Scheme::Dark => &image.dark,
        };
        file_url(path)
    });
    let layers = use_crossfade(url);
    let root = use_root_style();
    rsx! {
        Ds {
            sheet: Some(ds_shell::stylesheet()),
            appearance,
            system: environment.system,
            material: Material::Widget,
            extent: RootExtent::Viewport,
            blur: BlurState::Unavailable,
            surface: root.surface,
            user_style: root.user_style,
            AppStyle { css: WALLPAPER_CSS }
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
