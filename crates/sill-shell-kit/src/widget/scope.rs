//! The size of the `WidgetFrame` a component is drawn in, for content that fits itself to the
//! frame without the caller saying so (a `MonthGrid`'s `Auto` density). The frame
//! provides it once; a descendant reads it, and reads nothing outside a frame.

use crate::widget::kind::WidgetSize;
use dioxus::prelude::*;

/// The enclosing frame's size, as its subtree reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EnclosingFrame(WidgetSize);

/// Provide `size` to the frame's subtree, updating the provided value when it changes. Nothing
/// here reads the signal, so the write does not re-render the frame.
pub(crate) fn use_frame_provider(size: WidgetSize) {
    let mut provided = use_context_provider(|| Signal::new(EnclosingFrame(size)));
    if *provided.peek() != EnclosingFrame(size) {
        provided.set(EnclosingFrame(size));
    }
}

/// The size of the nearest enclosing `WidgetFrame`, or `None` outside one.
pub(crate) fn use_enclosing_frame() -> Option<WidgetSize> {
    try_use_context::<Signal<EnclosingFrame>>().map(|provided| provided().0)
}
