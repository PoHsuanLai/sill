//! A question from the compositor: a program wants a power and the person decides
//! (`quire_shell_v1` version 5, `permission_ask`). Pure data: the text of the program is made safe
//! to hold and draw once, here, because it comes from another process's file name.

use casement_protocols::vocab::{AskId as ConsentId, Memory, Power, SubjectKind};

use crate::untrusted::for_display;

/// The longest subject kept, in characters. The protocol carries at most 4096 bytes; a path longer
/// than this shows its beginning and an ellipsis.
pub const SUBJECT_MAX_CHARS: usize = 4096;

/// Which program is asking, as far as the compositor can say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentSubject {
    /// How well the compositor knows it.
    pub kind: SubjectKind,
    /// Its image path, or the sandboxed app's id; empty when none is known. Cleaned for display.
    pub text: String,
}

/// One question from the compositor, as the shell holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consent {
    /// The question's number: the answer and the withdrawal name it.
    pub ask: ConsentId,
    /// What the program wants to do.
    pub power: Power,
    /// Who wants it.
    pub subject: ConsentSubject,
    /// Whether the answer is kept (`Remembered`) or the program will be asked again
    /// (`ThisTimeOnly`, which offers Allow Once).
    pub memory: Memory,
}

impl Consent {
    /// The question the wire carries, with the subject's text made safe to show.
    pub fn from_wire(
        ask: ConsentId,
        power: Power,
        kind: SubjectKind,
        subject: &str,
        memory: Memory,
    ) -> Consent {
        Consent {
            ask,
            power,
            subject: ConsentSubject {
                kind,
                text: for_display(subject, SUBJECT_MAX_CHARS),
            },
            memory,
        }
    }
}
