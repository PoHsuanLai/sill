//! The wallpaper (design/20 §1.4): one per output, Background layer, no input, no blur, the
//! prepared image with a light/dark cross-fade over `--t-big`, rendered only on change.
//!
//! No `Material::Window`: design/20 §1.4 gives the wallpaper no material at all ("opaque
//! image"), and quire's eight materials (`ds::prelude::Material`) have no such variant. `Window` is the
//! one wrong choice regardless — its own frame paints the Space's gradient
//! (`--m-tint-solid:var(--f-grad)`, `materials_css.rs`), which is exactly the per-workspace tint
//! design/21 §8 says the wallpaper must not have. `Material::Widget` is used instead (closest
//! in kind: a background-level surface, not a bar/menu/sheet one) with `BlurState::Unavailable`
//! (design's "blur none"): its own paper tint never shows, since the two image layers (or, with
//! no source, `.sill-wallpaper`'s own `--paper`) cover the root completely. A quire gap: no
//! material variant paints nothing at all.

mod crossfade;
mod io;
mod layer;
mod model;
mod surface;
mod view;

pub(crate) use model::url_for;
pub use surface::Wallpaper;
pub use view::WALLPAPER_CSS;
