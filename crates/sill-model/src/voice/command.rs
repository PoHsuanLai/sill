//! What tells the voice service things, what happens in it, and what that asks of the outside.

use voice_wire::{UtteranceEnd, VoiceEvent as Frame, VoiceRefusal, VoiceTrigger};

use super::model::{AskScope, ConsentAnswer, VoiceLink, VoiceParams};
use crate::companion::AskInput;

/// What the daemon and the surfaces tell the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceCommand {
    /// The settings changed.
    Params(VoiceParams),
    /// The hold began, over `scope`.
    Start {
        /// What was focused.
        scope: AskScope,
        /// What began it.
        trigger: VoiceTrigger,
    },
    /// The key was let go: finish and send.
    Stop,
    /// Escape or other input: discard everything.
    Cancel,
    /// The note was read.
    Dismiss,
    /// The consent sheet was answered.
    Answer(ConsentAnswer),
}

/// What happens in the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceEvent {
    /// The service was told something.
    Told(VoiceCommand),
    /// The shell's seat was taken or given up.
    Link(VoiceLink),
    /// `Begin` returned: the utterance exists.
    Began,
    /// `voiced` refused `Begin`.
    Refused(VoiceRefusal),
    /// `Begin` could not be asked.
    Unreachable,
    /// A frame of the utterance's stream.
    Frame(Frame),
    /// The utterance's `Ended` signal (it carries no words).
    Ended(UtteranceEnd),
    /// The stream closed.
    StreamClosed,
}

/// What the service must do outside the pure state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceEffect {
    /// Call `Voice1.Begin`.
    Begin(VoiceTrigger),
    /// Call `Utterance.Release`.
    Release,
    /// Call `Utterance.Cancel`.
    Cancel,
    /// Write the answer to the sheet into the settings.
    Record(ConsentAnswer),
    /// Give the companion the question.
    Ask(AskInput),
}
