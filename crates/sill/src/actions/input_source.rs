//! Next Input Source: handed to the input source service, which finds the running input-method
//! framework when asked. From a surface nobody waits for the answer and a refusal is the
//! service's to log ([`next`]); `sill do` waits for it ([`ask`]) and is told in a plain sentence.

use std::sync::mpsc::SyncSender;
use std::time::Duration;

use sill_ipc::response::{FailureKind, Response};
use sill_model::input_source::{InputCommand, InputSwitch};
use sill_ui_kit::services::Services;
use tokio::runtime::Handle;

use crate::ipc::dispatch::{Reach, failed, sent};

/// How long `sill do` waits for the service's answer: inside the IPC server's reply time.
const ANSWER_WITHIN: Duration = Duration::from_millis(1500);

/// Switch to the next input source.
pub fn next(reach: &Reach) -> Response {
    sent(
        reach.services.input_source.control.send(InputCommand::Next),
        "input source",
    )
}

/// Switch to the next input source and answer `reply` with how it went, on `runtime`.
pub fn ask(services: &Services, runtime: &Handle, reply: SyncSender<Response>) {
    let control = services.input_source.control.clone();
    let mut state = services.input_source.state.clone();
    runtime.spawn(async move {
        state.mark_unchanged();
        let response = match control.send(InputCommand::Next) {
            Err(error) => failed(FailureKind::Internal, format!("input source: {error}")),
            Ok(()) => match tokio::time::timeout(ANSWER_WITHIN, state.changed()).await {
                Ok(Ok(())) => response_of(&state.borrow()),
                Ok(Err(_)) | Err(_) => failed(
                    FailureKind::Timeout,
                    "The input source did not answer.".to_owned(),
                ),
            },
        };
        // A client that has gone away cannot be told.
        let _gone_is_dropped = reply.send(response);
    });
}

/// What `sill do next-input-source` is told of how the switch went.
fn response_of(now: &InputSwitch) -> Response {
    match now {
        InputSwitch::Switched(_) => Response::Done,
        InputSwitch::Refused(refusal) => {
            failed(FailureKind::Refused, refusal.sentence().to_owned())
        }
        InputSwitch::Idle => failed(
            FailureKind::Internal,
            "The input source did not answer.".to_owned(),
        ),
    }
}
