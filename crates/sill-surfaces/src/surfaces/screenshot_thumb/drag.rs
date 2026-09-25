//! Dragging the thumbnail out as a file: the hook, frozen, waiting on shell-host.
//!
//! A drag out of a Wayland surface is a `wl_data_source` offering `text/uri-list` (the file's
//! `file://` URI) and `wl_data_device.start_drag` with the press's serial and an icon surface.
//! shell-host has the drop side designed (its FINDINGS F35, gap G4) and no drag source:
//! `SurfaceHandle::start_drag(mimes, icon)` is named there as separate, unbuilt work. Until it
//! lands, [`start_file_drag`] answers `Unsupported`, the thumbnail treats a drag past the swipe
//! threshold to the left or up as nothing, and the gap is sill G120 (FINDINGS "M10 freeze").

use std::path::Path;

/// Whether a drag started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DragStart {
    /// The compositor has the drag; a later drop or cancel ends it.
    Started,
    /// shell-host has no drag source yet (G4, sill G120).
    Unsupported,
}

/// Start dragging `file` out of the surface as `text/uri-list`, from the press that began the
/// gesture. Lane c calls it; the body changes when shell-host's `start_drag` exists.
pub fn start_file_drag(file: &Path) -> DragStart {
    let _ = file;
    DragStart::Unsupported
}

#[cfg(test)]
mod tests {
    use super::{DragStart, start_file_drag};

    #[test]
    fn no_drag_source_yet() {
        assert_eq!(
            start_file_drag(std::path::Path::new("/tmp/Screenshot_1.png")),
            DragStart::Unsupported
        );
    }
}
