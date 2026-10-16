//! The effects end of one question's card: the person's outcome sent to the capture service. The
//! card is keyed by the question's id, so each question starts afresh. A stopped service has no
//! one to answer; the sheet goes with it.

use dioxus::prelude::{Callback, use_callback};
use sill_model::capture_pick::{AskId, CaptureCommand, PermissionOutcome};
use sill_services::capture_pick::CaptureService;
use sill_ui_kit::hooks::use_control;

/// Ends question `id` with the outcome it is called with.
pub(super) fn use_end(id: AskId) -> Callback<PermissionOutcome> {
    let control = use_control::<CaptureService>();
    use_callback(move |outcome: PermissionOutcome| {
        let _ = control.send(CaptureCommand::PermissionAnswer(id, outcome));
    })
}
