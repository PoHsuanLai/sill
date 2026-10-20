//! The first-use voice consent sheet (voice.md section 5, rule 9): what the person is asked
//! before the microphone ever opens. One Overlay per output, kept warm and hidden like the
//! confirmation sheet, shown on the sheets' output (the focused
//! window's, latched while a question is up) while the voice service
//! says `Asking`. It draws one sentence on quire's `Sheet` with two buttons, Not Now and Allow,
//! and answers through `VoiceCommand::Answer`; Escape is Not Now. Whatever the answer, the
//! daemon writes it into `voice.consent` of the settings file, which `voiced` reads on every hold.

mod surface;
mod view;

pub use surface::VoiceConsentSheet;

/// The consent sheet's stylesheet (lint-clean under `Profile::Strict`,
/// `crates/sill/tests/lint.rs`).
pub const VOICE_CONSENT_CSS: &str = include_str!("style.css");
