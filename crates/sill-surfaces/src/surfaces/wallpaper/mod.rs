//! The wallpaper (design/20 §1.4): one per output, Background layer, no input, no blur, the
//! prepared image with a light/dark cross-fade over `--t-big`, rendered only on change.
//!
//! No `Material::Window`: design/20 §1.4 gives the wallpaper no material at all ("opaque
//! image"), and quire's eight materials (`ds::Material`) have no such variant. `Window` is the
//! one wrong choice regardless — its own frame paints the Space's gradient
//! (`--m-tint-solid:var(--f-grad)`, `materials_css.rs`), which is exactly the per-workspace tint
//! design/21 §8 says the wallpaper must not have. `Material::Widget` is used instead (closest
//! in kind: a background-level surface, not a bar/menu/sheet one) with `BlurState::Unavailable`
//! (design's "blur none"): its own paper tint never shows, since the two image layers (or, with
//! no source, `.sill-wallpaper`'s own `--paper`) cover the root completely. Recorded in
//! FINDINGS.md ("M2 wallpaper") as a quire gap: no material variant paints nothing at all.

mod crossfade;
pub mod layer;

pub use layer::wallpaper_layer;

use dioxus::prelude::*;
use ds::{BlurState, Ds, Material};
use shell_host::OutputInfo;
use sill_services::{OutputName, WallpaperImage, Wallpapers};

use crate::hooks::{use_appearance, use_wallpapers};
use crossfade::Layers;

/// The wallpaper's own stylesheet (lint-clean under `Profile::Strict`, `tests/wallpaper_lint.rs`).
pub(crate) const WALLPAPER_CSS: &str = include_str!("../../style/wallpaper.css");

/// `wallpapers`' entry for `output`, if it has one yet (a fresh output, or a source that never
/// resolved, has none).
fn image_for<'a>(
    wallpapers: &'a Wallpapers,
    output: Option<&OutputName>,
) -> Option<&'a WallpaperImage> {
    let output = output?;
    wallpapers
        .outputs
        .iter()
        .find(|image| &image.output == output)
}

/// A cache path as the `file://` URL shell-host's `LocalNet` reads directly (mirrors
/// `sill_services::tray::icon::named_icon_file_url`).
fn file_url(path: &std::path::Path) -> String {
    format!("file://{}", path.display())
}

/// The wallpaper on `output`: an `Ds` root over two cross-fading image layers showing the
/// cached file for the resolved scheme (`ds::resolve`, the same call `Ds` makes internally, so
/// the two never disagree about which scheme is current).
#[component]
pub fn Wallpaper(output: OutputInfo) -> Element {
    let environment = use_appearance()();
    let wallpapers = use_wallpapers()();
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

/// The persistent A/B state ([`Layers`]) a `Ds` frame keeps the same way (`root/ds.rs`'s
/// `use_frame_layers`): held outside any signal, since `Wallpaper` already re-renders whenever
/// `use_wallpapers` or `use_appearance` changes, and nothing else needs to read it.
fn use_crossfade(
    image: Option<String>,
) -> [(&'static str, Option<&'static str>, Option<String>); 2] {
    let mut layers = use_hook(|| CopyValue::new(Layers::starting_at(image.clone())));
    let next = layers.peek().clone().show(image);
    let rendered = next.render();
    layers.set(next);
    rendered
}
