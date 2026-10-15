//! The effects end of the panel: follow the link onto the stage (and so the surface's mapping),
//! and read the apps the desktop lists.

use dioxus::prelude::*;
use shell_host::{OutputInfo, use_watch};
use sill_model::desktop::ForceQuitRow;
use sill_ui_kit::links::{ForceQuitLink, ForceQuitOpening};
use sill_ui_kit::sheet_stage::{SheetStage, use_sheet_stage};

/// The stage on `output`, following the link, and the surface's mapping following the stage.
/// Nothing to map without a host (a test, an SSR render).
pub(super) fn use_stage(output: &OutputInfo) -> Signal<SheetStage<ForceQuitOpening>> {
    let link = use_context::<ForceQuitLink>();
    let shown = use_watch(link.subscribe());
    let id = output.id;
    use_sheet_stage(move || {
        shown
            .read()
            .opening()
            .filter(|opening| opening.output == id)
            .copied()
    })
}

/// The apps the desktop lists now.
pub(super) fn use_rows() -> ReadSignal<Vec<ForceQuitRow>> {
    let link = use_context::<ForceQuitLink>();
    use_watch(link.subscribe_rows())
}
