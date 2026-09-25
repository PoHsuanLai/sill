//! The plate a full-width module draws its own content on (the levels, Now Playing, Appearance,
//! Battery), spanning both columns of quire's `ModuleGrid`. quire's `ModuleTile` is a toggle
//! with a glyph, a title and a status and holds no content of its own, so this box takes the
//! tile's plate in the control center's sheet (`--r-tile`, `--surface-2`, a hairline); it goes
//! when quire's level control (Q83) or a content-holding tile brings its own (FINDINGS Q84).

use dioxus::prelude::*;

/// `children` on a full-width module plate.
#[component]
pub(crate) fn ModuleBox(children: Element) -> Element {
    rsx! {
        div { class: "sill-cc-module", {children} }
    }
}
