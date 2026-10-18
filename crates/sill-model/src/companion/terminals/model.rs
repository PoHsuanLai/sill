//! What the panel knows about sessions in terminals.

use companion_wire::TerminalSession;
use prov::SessionId;

/// A bring back the companion would not do, with its sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedBringBack {
    /// The session.
    pub session: SessionId,
    /// What the companion said, as it is.
    pub sentence: String,
}

/// The sessions in terminals.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerminalsModel {
    /// The sessions the companion lists, in its order.
    pub sessions: Vec<TerminalSession>,
    /// The session most recently brought back: its conversation is the one the panel opens.
    pub brought_back: Option<SessionId>,
    /// The bring back that was refused, until the person tries again or the list changes.
    pub refused: Option<RefusedBringBack>,
}

impl TerminalsModel {
    /// The session `id`, when the companion lists it.
    pub fn session(&self, id: &SessionId) -> Option<&TerminalSession> {
        self.sessions.iter().find(|s| s.session == *id)
    }

    /// The sentence to show under `id`'s row, when its bring back was refused.
    pub fn refusal_of(&self, id: &SessionId) -> Option<&str> {
        self.refused
            .as_ref()
            .filter(|refused| refused.session == *id)
            .map(|refused| refused.sentence.as_str())
    }
}
