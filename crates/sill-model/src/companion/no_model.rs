//! The turn that met no model: the answer says so in its own box, with a button that opens
//! Settings at the assistant's model. Nothing opens until the person presses it.

use companion_wire::{AnswerBody, RefusalWire};

/// What the answer says when it was refused for want of a model.
pub const NO_MODEL_LINE: &str = "Choose a model to use the assistant.";

/// The button under that line.
pub const CHOOSE_MODEL_LABEL: &str = "Choose Model\u{2026}";

/// Whether the companion refused `body` for want of a model.
pub fn lacks_model(body: &AnswerBody) -> bool {
    matches!(body, AnswerBody::Refused(RefusalWire::NoModel))
}
