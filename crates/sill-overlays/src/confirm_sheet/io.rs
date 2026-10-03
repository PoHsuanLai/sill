//! The effects end of one question's card: the gate on quire's clock, and the answer sent to the
//! intents service. The card is keyed by the question's id, so each question starts its own gate
//! (and its arming) afresh.

use dioxus::prelude::*;
use ds::machine::{MachineRef, use_machine};
use sill_model::intents::{ConfirmId, IntentsCommand};
use sill_services::IntentsService;
use sill_ui_kit::hooks::use_control;

use super::gate::{AnswerGate, GateIn, GateOut, GateParams};

/// The gate for question `id`: started when the card first draws, sending its answer to the
/// service. A stopped service has no one to answer; the sheet goes with it.
pub(super) fn use_gate(id: ConfirmId) -> MachineRef<AnswerGate> {
    let control = use_control::<IntentsService>();
    let gate = use_machine::<AnswerGate>(GateParams::default(), move |out| match out {
        GateOut::Answer(choice) => {
            let _ = control.send(IntentsCommand::Answer {
                id: id.clone(),
                choice,
            });
        }
    });
    use_effect(move || gate.send(GateIn::Shown));
    gate
}
