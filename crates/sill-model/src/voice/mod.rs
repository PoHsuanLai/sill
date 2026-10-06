//! Hold-to-talk (voice.md section 4.6): the person holds the key, `voiced` listens and sends the
//! words back on a stream, and what was said goes to the companion as the question.
//!
//! The pure half of the voice service. [`step`] keeps the published [`Voice`]: where the hold
//! stands, the words heard so far, the refusal the person is told about and whether the consent
//! sheet is up. `sill-services` does the bus and carries out each [`VoiceEffect`].

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;
mod words;

pub use command::{VoiceCommand, VoiceEffect, VoiceEvent};
pub use model::{
    AskScope, ConsentAnswer, ConsentSheet, Heard, HoldToTalk, Notice, Phase, Then, Voice,
    VoiceConsent, VoiceLink, VoiceParams, Words,
};
pub use step::step;
pub use voice_wire::VoiceTrigger;
pub use words::{listening_line, notice_line};
