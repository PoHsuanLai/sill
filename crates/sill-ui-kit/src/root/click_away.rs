//! ClickAway: the transparent layer under a surface's panel that takes every press the panel
//! does not, so a panel can close when the person clicks elsewhere. A sheet dims nothing and
//! catches no click (design/30 Part 4), and click-outside belongs to the host: until shell-host
//! delivers an outside press, this is the one place sill covers the output for it.
//!
//! Its box fills the nearest positioned ancestor, which is the surface's root.

use dioxus::prelude::*;

/// A press on the layer calls `onpress`; `mounted` hears the element once it is in the document
/// (the desktop grid is measured from it).
#[component]
pub fn ClickAway(
    onpress: EventHandler<()>,
    #[props(default)] mounted: Option<EventHandler<MountedEvent>>,
) -> Element {
    rsx! {
        div {
            "data-layer": "click-away",
            style: "position:absolute;top:0;right:0;bottom:0;left:0",
            onmounted: move |event| {
                if let Some(mounted) = mounted {
                    mounted.call(event);
                }
            },
            onpointerdown: move |_| onpress.call(()),
        }
    }
}
