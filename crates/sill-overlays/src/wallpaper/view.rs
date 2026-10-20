//! The wallpaper's render: an `Ds` root over two cross-fading image layers, and the desktop's
//! menu at a press of the secondary button.

use dioxus::prelude::*;
use ds::prelude::*;
use ds::style::appearance::blur::BlurState;
use shell_host::OutputInfo;
use sill_model::ids::OutputName;

use super::io::use_crossfade;
use super::menu::{secondary_press, use_desktop_menu};
use super::model::url_for;
use sill_services::WallpaperService;
use sill_ui_kit::hooks::{use_appearance, use_service};
use sill_ui_kit::menu::use_popup_look;
use sill_ui_kit::root::use_root_style;

/// The wallpaper's own stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const WALLPAPER_CSS: &str = include_str!("style.css");

/// The wallpaper on `output`: an `Ds` root over two cross-fading image layers showing the
/// cached file for the resolved scheme (`ds::prelude::resolve`, the same call `Ds` makes internally, so
/// the two never disagree about which scheme is current).
pub(super) fn wallpaper(output: OutputInfo) -> Element {
    let environment = use_appearance()();
    let wallpapers = use_service::<WallpaperService>()();
    let on = output.name.map(OutputName);
    let appearance = environment.settings.appearance.appearance();
    let resolved = resolve(appearance, SpaceLook::default().theme, environment.system);
    let url = url_for(&wallpapers, on.as_ref(), resolved.scheme);
    let layers = use_crossfade(url);
    let root = use_root_style();
    let menu = use_desktop_menu(use_popup_look());
    rsx! {
        Ds {
            sheet: Some(sill_shell_kit::stylesheet()),
            appearance,
            system: environment.system,
            material: Material::Widget,
            extent: RootExtent::Viewport,
            blur: BlurState::Unavailable,
            surface: root.surface,
            user_style: root.user_style,
            AppStyle { css: WALLPAPER_CSS }
            div {
                class: "sill-wallpaper",
                onmousedown: move |event| secondary_press(&event, menu),
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
