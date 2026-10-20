//! Next Input Source: handed to the input source service, which finds the running input-method
//! framework when asked. A refusal (no framework, one the shell cannot switch) is the service's
//! to log; nobody waits for the answer.

use sill_ipc::response::Response;
use sill_model::input_source::InputCommand;

use crate::ipc::dispatch::{Reach, sent};

/// Switch to the next input source.
pub fn next(reach: &Reach) -> Response {
    sent(
        reach
            .services
            .input_source
            .control
            .send(InputCommand::Next),
        "input source",
    )
}
