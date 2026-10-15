//! The effects end of one question's card: the gate on quire's clock, and the answer sent to the
//! intents service. The card is keyed by the question's id, so each question starts its own gate
//! (and its arming) afresh.

use dioxus::prelude::{Callback, use_callback};
use ds::machine::{MachineRef, use_machine};
use sill_model::intents::{ConfirmId, IntentsCommand, StandingGrantId};
use sill_services::IntentsService;
use sill_ui_kit::hooks::use_control;

use super::gate::{AnswerGate, GateOut, GateParams};

/// The gate for question `id`: arming from the moment the card first draws (its first state is
/// seeded, with the arming deadline on the hook's clock), sending its answer to the service. A
/// stopped service has no one to answer; the sheet goes with it.
pub(super) fn use_gate(id: ConfirmId) -> MachineRef<AnswerGate> {
    let control = use_control::<IntentsService>();
    let params = GateParams::default();
    use_machine(
        move |now| AnswerGate::arming(now, &params),
        params,
        || (),
        move |out, _| match out {
            GateOut::Answer(choice) => {
                let _ = control.send(IntentsCommand::Answer {
                    id: id.clone(),
                    choice,
                });
            }
        },
    )
}

/// Where Undo goes: the entry that made the sheet ask is taken away. A stopped service has no one
/// to tell; the sheet goes with it.
pub(super) fn use_undo() -> Callback<StandingGrantId> {
    let control = use_control::<IntentsService>();
    use_callback(move |id: StandingGrantId| {
        let _ = control.send(IntentsCommand::UndoLimit(id));
    })
}
