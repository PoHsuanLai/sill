//! The effects end of one request's card: the person's outcome sent to the capture service. The
//! card is keyed by the request's id, so each request starts afresh. A stopped service has no one
//! to answer; the sheet goes with it.

use dioxus::prelude::{Callback, use_callback};
use sill_model::capture_pick::{AskId, CaptureCommand, Outcome};
use sill_services::capture_pick::CaptureService;
use sill_ui_kit::hooks::use_control;

/// Ends request `id` with the outcome it is called with.
pub(super) fn use_end(id: AskId) -> Callback<Outcome> {
    let control = use_control::<CaptureService>();
    use_callback(move |outcome: Outcome| {
        let _ = control.send(CaptureCommand::Answer(id, outcome));
    })
}
