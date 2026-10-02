//! A root that starts over when the scheme changes.
//!
//! Blitz bakes a glyph's and a text run's colour in when it builds the box, and a colour-only
//! restyle never builds it again, so a root that is switched between Light and Dark while it is up
//! keeps drawing its glyphs and text in the old scheme's ink (dark Trash and status icons on a dark
//! dock and bar). shell-host has no hook to rebuild the boxes (quire's `ds_blitz::seam::follow_scheme`
//! does it for the harness and for windowed hosts), so a surface wraps its `Ds` in this: the scheme
//! is the key of a one-item list, and a new key makes Dioxus build the root again, boxes included.

use dioxus::prelude::*;

/// `children` (a `Ds` root), built again whenever `scheme` changes.
#[component]
pub fn SchemeKeyed(scheme: String, children: Element) -> Element {
    rsx! {
        for scheme in [scheme] {
            Fragment { key: "{scheme}", {children.clone()} }
        }
    }
}
