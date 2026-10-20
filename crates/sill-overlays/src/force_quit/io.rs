//! The effects end of the panel: follow the link onto the stage (and so the surface's mapping),
//! and list the apps of the desktop's view.

use dioxus::prelude::*;
use shell_host::{OutputInfo, use_watch};
use sill_model::desktop::{ForceQuitRow, force_quit_rows};
use sill_ui_kit::links::{ForceQuitLink, ForceQuitOpening};
use sill_ui_kit::sheet_stage::{SheetStage, use_sheet_stage};

/// The stage on `output`, following the link, and the surface's mapping following the stage.
/// Nothing to map without a host (a test, an SSR render).
pub(super) fn use_stage(output: &OutputInfo) -> Signal<SheetStage<ForceQuitOpening>> {
    let link = use_context::<ForceQuitLink>();
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

/// The apps the desktop lists now: its view, followed, as the panel's rows.
pub(super) fn use_rows() -> Memo<Vec<ForceQuitRow>> {
    let link = use_context::<ForceQuitLink>();
    let desktop = use_watch(link.subscribe_desktop());
    use_memo(move || force_quit_rows(&desktop.read()))
}
