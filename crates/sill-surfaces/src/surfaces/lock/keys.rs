//! The lock prompt's keys (design/20 §1.9, design/06 "Escape clears field"): pure.

use super::view::PromptState;

/// A key the prompt acts on; everything else is typing, which quire's field handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockKey {
    Enter,
    Escape,
}

/// Whether the field holds anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldText {
    Empty,
    Typed,
}

/// What a key does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockKeyOut {
    /// Send the field's text as `AuthCommand::Submit` and clear the field.
    Submit,
    /// Empty the field.
    Clear,
    Nothing,
}

/// `key` with the field `field` in `state`: Enter submits typed text while `Idle` or `Wrong`,
/// nothing otherwise (an empty field, a check running, a lockout); Escape clears typed text.
pub fn lock_key(key: LockKey, field: FieldText, state: PromptState) -> LockKeyOut {
    todo!("M11 fill: a-lock ({key:?} {field:?} {state:?})")
}

#[cfg(test)]
mod tests {
    use super::{FieldText, LockKey, LockKeyOut, lock_key};
    use crate::surfaces::lock::view::PromptState;

    #[test]
    #[ignore = "M11 fill: a-lock"]
    fn keys() {
        let cases = [
            (
                LockKey::Enter,
                FieldText::Typed,
                PromptState::Idle,
                LockKeyOut::Submit,
            ),
            (
                LockKey::Enter,
                FieldText::Typed,
                PromptState::Wrong,
                LockKeyOut::Submit,
            ),
            (
                LockKey::Enter,
                FieldText::Empty,
                PromptState::Idle,
                LockKeyOut::Nothing,
            ),
            (
                LockKey::Enter,
                FieldText::Typed,
                PromptState::Checking,
                LockKeyOut::Nothing,
            ),
            (
                LockKey::Enter,
                FieldText::Typed,
                PromptState::LockedOut { seconds: 30 },
                LockKeyOut::Nothing,
            ),
            (
                LockKey::Escape,
                FieldText::Typed,
                PromptState::Idle,
                LockKeyOut::Clear,
            ),
            (
                LockKey::Escape,
                FieldText::Empty,
                PromptState::Idle,
                LockKeyOut::Nothing,
            ),
        ];
        for (key, field, state, want) in cases {
            assert_eq!(
                lock_key(key, field, state),
                want,
                "{key:?} {field:?} {state:?}"
            );
        }
    }
}
