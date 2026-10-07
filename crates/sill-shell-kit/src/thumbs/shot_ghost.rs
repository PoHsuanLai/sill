//! ShotGhost: the screenshot thumbnail drawn small and translucent, for the host's drag icon
//! (design/04-COMPONENTS.md section 39). The host draws it on its own drag-icon
//! surface while it carries the file; quire only paints it. Smaller than the card so the drop
//! target under it stays readable, and translucent so what it is dropped on shows through. No
//! motion: a drag icon follows the pointer, it does not arrive.

use crate::thumbs::shot_frame::shot_frame;
use dioxus::prelude::*;
use ds::base::geometry::units::Px;
use ds::components::content::image_source::{ImageSize, ImageSource};
use ds::components::content::picture_fit::picture_style;
use ds::root::chrome::RootChrome;
use ds::root::common::Common;
use ds::root::surface::Surface;
use ds::style::appearance::material::Material;

/// The ghost's width: half the default card's.
const GHOST_WIDTH: Px = Px(120.0);

/// The drag ghost of a thumbnail showing `image` of `size`.
#[component]
pub fn ShotGhost(image: ImageSource, size: ImageSize, #[props(default)] common: Common) -> Element {
    let frame = shot_frame(GHOST_WIDTH, size);
    let class = common.class("ds-shot-ghost");
    let data = common.data_attributes();
    rsx! {
        Surface { material: Material::Toast, chrome: RootChrome::Transparent,
            div {
                class,
                id: common.id.clone(),
                "aria-hidden": "true",
                style: "width:{frame.card.width.0}px",
                onmounted: move |event| common.mounted(event),
                ..data,
                div { class: "ds-shot-plate", style: "height:{frame.card.height.0}px",
                    img { class: "ds-shot-image", alt: "", src: image.0, style: picture_style(frame.picture) }
                }
            }
        }
    }
}
