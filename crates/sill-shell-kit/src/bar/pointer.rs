//! BarPointer: the raw pointer events a bar item hands on to its owner (design/30 section 2.10).
//! A menu bar's session opens a menu on the press, switches on the pointer's entry and
//! completes on the release, and a workspace strip reorders by a press on one pill released over
//! another. A click is not enough for either, so the item passes these three events on itself,
//! where its owner used to wrap it in an element of its own to hear them.

use dioxus::prelude::*;
use std::fmt;

/// The handlers for an item's pointer events; each is optional.
#[derive(Clone, PartialEq, Default)]
pub struct BarPointer {
    /// A pointer button went down on the item.
    pub down: Option<EventHandler<PointerEvent>>,
    /// A pointer button came up on the item.
    pub up: Option<EventHandler<PointerEvent>>,
    /// The pointer came over the item.
    pub enter: Option<EventHandler<PointerEvent>>,
}

impl fmt::Debug for BarPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BarPointer")
            .field("down", &self.down.is_some())
            .field("up", &self.up.is_some())
            .field("enter", &self.enter.is_some())
            .finish()
    }
}

impl BarPointer {
    /// Hand a press to `down`.
    pub(crate) fn pressed(&self, event: PointerEvent) {
        if let Some(down) = &self.down {
            down.call(event);
        }
    }

    /// Hand a release to `up`.
    pub(crate) fn released(&self, event: PointerEvent) {
        if let Some(up) = &self.up {
            up.call(event);
        }
    }

    /// Hand an entry to `enter`.
    pub(crate) fn entered(&self, event: PointerEvent) {
        if let Some(enter) = &self.enter {
            enter.call(event);
        }
    }
}
