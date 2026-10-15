//! What a failed turn says when the call was refused as not allowed: a "never" the person set
//! says so plainly; the other denials keep the general words.

use companion_wire::{AnswerBody, AnswerPhase, RefusalWire};
use docket_core::DenyCode;

use super::answer_line;
use super::model_picker_tests::{held, task, wire};

#[test]
fn a_denied_call_says_never_when_the_person_said_never() {
    let cases = [
        ("never", DenyCode::Never, "You said never to do this"),
        ("outside the task", DenyCode::OutsideTask, "Not allowed"),
        ("not for an assistant", DenyCode::NotAllowed, "Not allowed"),
        ("needs the person", DenyCode::NeedsUser, "Not allowed"),
        ("refused before", DenyCode::Repeated, "Not allowed"),
    ];
    for (name, code, want) in cases {
        let refused = wire(
            AnswerPhase::Failed,
            AnswerBody::Refused(RefusalWire::NotAllowed(code)),
        );
        let state = held("archive the Work threads", refused);
        let (_, subtitle) = answer_line(&state.answers[&task()]);
        assert_eq!(subtitle, want, "{name}");
    }
}
