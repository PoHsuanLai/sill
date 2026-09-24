//! A `ds::HostMeasure` for documents shell-host runs: quire's rect reads (`use_rect`, every
//! floating component's anchor) go through it, and without one they call dioxus-native-dom's
//! `get_client_rect`, which borrows the document and panics ("RefCell already borrowed") when
//! the task is polled inside the renderer's own pass. shell-host cannot provide it (it does not
//! depend on quire) and ds-native's is private, so the bar provides its own, the same read
//! ds-native makes (FINDINGS "M1 bar-ui", Q7).

use dioxus::prelude::MountedData;
use dioxus_native_dom::NodeHandle;
use ds::{HostMeasure, Measured, Point, Px, Rect, Size};

/// The measurer the bar and its popup provide as context.
pub(crate) const MEASURE: HostMeasure = HostMeasure(measure);

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
