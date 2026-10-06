//! What the person reads about voice: the line for a refusal and the placeholder while listening.

use voice_wire::{VoiceFault, VoiceRefusal};

use super::model::{Notice, Phase};

/// The plain sentence for `notice`; empty for none.
pub fn notice_line(notice: Notice) -> &'static str {
    match notice {
        Notice::None => "",
        Notice::Refused(VoiceRefusal::MicUnavailable)
        | Notice::Failed(VoiceFault::MicUnavailable) => "No microphone is available.",
        Notice::Failed(VoiceFault::MicDenied) => "Microphone access was denied.",
        Notice::Refused(VoiceRefusal::Disabled) => "Voice is turned off in Settings.",
        Notice::Refused(VoiceRefusal::NoModel) | Notice::Failed(VoiceFault::NoModel) => {
            "No speech model is installed."
        }
        Notice::Refused(VoiceRefusal::Busy) => "Voice is busy.",
        Notice::Refused(VoiceRefusal::NotAllowed) => "Voice is not allowed here.",
        Notice::Refused(VoiceRefusal::NeedsConsent) => "Voice needs your permission first.",
        Notice::Unreachable => "Voice is not running.",
        Notice::NothingHeard => "Didn't catch that.",
        Notice::Failed(VoiceFault::Engine(_) | VoiceFault::Refused(_)) => {
            "Couldn't turn speech into text."
        }
    }
}

/// What the prompt's field says while voice is on it: the words so far, "Listening" before any,
/// the notice after a hold that failed; `None` when voice has nothing to say.
pub fn listening_line(phase: &Phase, notice: Notice) -> Option<String> {
    match phase {
        Phase::Opening(_) => Some("Listening…".to_owned()),
        Phase::Live(heard) | Phase::Finishing(heard) => Some(match heard.text() {
            text if text.is_empty() => "Listening…".to_owned(),
            text => text,
        }),
        Phase::Idle => match notice_line(notice) {
            "" => None,
            line => Some(line.to_owned()),
        },
    }
}
