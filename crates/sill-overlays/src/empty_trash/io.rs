//! The effects end of the question: follow the link onto the stage (and so the surface's
//! mapping), and write the person's answer to it.

use dioxus::prelude::*;
use shell_host::{OutputInfo, use_watch};
use sill_ui_kit::links::{EmptyTrashLink, EmptyTrashOpening};
use sill_ui_kit::sheet_stage::{SheetStage, use_sheet_stage};

/// The stage on `output`, following the link, and the surface's mapping following the stage.
pub(super) fn use_stage(output: &OutputInfo) -> Signal<SheetStage<EmptyTrashOpening>> {
    let link = use_context::<EmptyTrashLink>();
    let shown = use_watch(link.subscribe());
    let id = output.id;
    use_sheet_stage(super::surface::PRESET, move || {
        shown
            .read()
            .opening()
            .filter(|opening| opening.output == id)
            .copied()
    })
}
