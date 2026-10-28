//! The eyedropper's render: one transparent viewport root holding, while a picture of this output
//! is up, a key target (focused per question, so Escape reaches it) over the still picture, and
//! the round magnifier that follows the pointer: the same picture enlarged behind a round clip,
//! the pixel under the pointer ringed in the middle. A press picks the pixel under it (the
//! primary button) or cancels (the other); Escape cancels.
//!
//! [`super::io`] follows the link and sends the answer, [`super::model`] places the magnifier;
//! this file only draws.

use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use ds::focus::soon::focus_soon;
use ds::prelude::*;
use shell_host::OutputInfo;
use sill_model::capture_pick::{AskId, Rgb8, loupe_at, pick_at};
use sill_model::desktop::OutputShot;
use sill_ui_kit::links::EyedropperLink;
use sill_ui_kit::root::SurfaceRoot;

use super::io::use_sampler;
use super::model::{cell_style, loupe_style, surface_of, zoom_style};

/// The eyedropper's stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const EYEDROPPER_CSS: &str = include_str!("style.css");

pub(super) fn color_sampler(output: OutputInfo) -> Element {
    let link = use_context::<EyedropperLink>();
    let sampler = use_sampler(&link, &output);
    let ended = use_callback({
        let sampler = sampler.clone();
        move |(id, color): (AskId, Option<Rgb8>)| sampler.answer(id, color)
    });
    let up = sampler.shown().zip(surface_of(&output));
    rsx! {
        SurfaceRoot {
            output,
            material: Material::Osd,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            AppStyle { css: EYEDROPPER_CSS }
            if let Some(((id, shot), surface)) = up {
                Dropper {
                    key: "{id.0}",
                    id,
                    shot,
                    surface,
                    ended,
                }
            }
        }
    }
}

/// One question's picture, magnifier and keys.
#[component]
fn Dropper(
    id: AskId,
    shot: OutputShot,
    surface: (u32, u32),
    ended: Callback<(AskId, Option<Rgb8>)>,
) -> Element {
    // Encoded once per question: the picture is large and never changes while it is up.
    let source = use_hook({
        let png = shot.png.clone();
        move || ImageSource::png(&png).0
    });
    let mut pointer = use_signal(|| None::<(f64, f64)>);
    let loupe = pointer().and_then(|at| loupe_at(shot.size, surface, at));
    rsx! {
        div {
            class: "sill-eyedrop",
            tabindex: "0",
            onmounted: move |event: MountedEvent| focus_soon(event.data()),
            onkeydown: move |event: KeyboardEvent| {
                if event.key() == Key::Escape {
                    ended.call((id, None));
                }
            },
            onpointermove: move |event: PointerEvent| pointer.set(Some(at_of(&event))),
            onpointerleave: move |_| pointer.set(None),
            onpointerdown: move |event: PointerEvent| match event.trigger_button() {
                Some(MouseButton::Secondary) => ended.call((id, None)),
                Some(MouseButton::Primary) | None => {
                    let picked = pick_at(&shot, surface, at_of(&event));
                    if let Some(colour) = picked {
                        ended.call((id, Some(colour)));
                    }
                }
                Some(_) => {}
            },
            img { class: "sill-eyedrop-still", src: "{source}", alt: "" }
            if let Some(loupe) = loupe {
                div { class: "sill-eyedrop-loupe", style: loupe_style(&loupe),
                    img {
                        class: "sill-eyedrop-zoom",
                        src: "{source}",
                        alt: "",
                        style: zoom_style(&loupe),
                    }
                    div { class: "sill-eyedrop-cell", style: cell_style() }
                }
            }
        }
    }
}

/// Where `event` is on the surface, in logical pixels.
fn at_of(event: &PointerEvent) -> (f64, f64) {
    let at = event.client_coordinates();
    (at.x, at.y)
}
