//! A `ds::HostMeasure` for documents shell-host runs: quire's rect reads (`use_rect`, every
//! floating component's anchor) go through it, and without one they call dioxus-native-dom's
//! `get_client_rect`, which borrows the document and panics ("RefCell already borrowed") when
//! the task is polled inside the renderer's own pass. shell-host cannot provide it (it does not
//! depend on quire) and ds-native's is private (Q10), so sill provides one: the daemon puts
//! [`MEASURE`] on every surface's `Root` context, and a surface that opens a popup puts it on
//! the popup's `Root` too (a popup is its own `VirtualDom` and inherits nothing). Moved here from
//! the bar at the M2 freeze amendment (FINDINGS F54); the read is the one ds-native makes.

use dioxus::prelude::MountedData;
use dioxus_native_dom::NodeHandle;
use ds::{HostMeasure, Measured, Point, Px, Rect, Size};

/// The measurer every root is given as context (`Root::context(MEASURE)`).
pub const MEASURE: HostMeasure = HostMeasure(measure);

/// `element`'s border box, if the document is free and the element is a Blitz node; busy
/// while the renderer holds the document, so the reader waits a frame.
fn measure(element: &MountedData) -> Measured {
    let Some(handle) = element.downcast::<NodeHandle>() else {
        return Measured::Unknown;
    };
    let Some(doc) = handle.try_doc() else {
        return Measured::Busy;
    };
    match doc.get_client_bounding_rect(handle.node_id()) {
        Some(found) => Measured::At(Rect {
            origin: Point {
                x: Px(found.x as f32),
                y: Px(found.y as f32),
            },
            size: Size {
                width: Px(found.width as f32),
                height: Px(found.height as f32),
            },
        }),
        None => Measured::Unknown,
    }
}
