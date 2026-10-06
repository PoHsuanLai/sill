//! What the voice service publishes: whether it reaches `voiced`, what the settings allow, where
//! the hold stands and what the person has been told.

use ds_core::word::Word;
use prov::{AppName, UnixSeconds};
use serde::{Deserialize, Serialize};
use sill_launcher::WindowKey;
use voice_wire::{VoiceFault, VoiceRefusal};

/// What the person said. `Debug` hides it: words must not reach logs or panics.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Words(pub String);

impl std::fmt::Debug for Words {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Words(<{} bytes>)", self.0.len())
    }
}

/// `voice.hold_to_talk`: whether holding the command key talks to the companion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum HoldToTalk {
    /// Holding the command key listens.
    #[default]
    HoldCommand,
    /// Nothing listens.
    Off,
}

/// `voice.consent`: the person's answer to the first-use sheet. `voiced` reads this very file, so
/// the serde form is its contract: the strings `"not_asked"` and `"declined"`, and the table
/// `{ given = <unix seconds> }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceConsent {
    /// The sheet has not been answered.
    #[default]
    NotAsked,
    /// Allowed, at this time.
    Given(UnixSeconds),
    /// Not Now: voice stays off until the setting changes.
    Declined,
}

/// The person's answer to the first-use sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsentAnswer {
    /// Allow: listening while the key is held.
    Allow,
    /// Not Now: voice stays off until the setting changes.
    NotNow,
}

/// What the settings say about voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoiceParams {
    /// Whether the key listens.
    pub hold_to_talk: HoldToTalk,
    /// The person's answer to the sheet.
    pub consent: VoiceConsent,
}

/// Whether the service reaches the bus as the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceLink {
    /// The shell's seat is not held yet: nothing can begin.
    #[default]
    Passive,
    /// Connected as the shell.
    Ready,
}

/// What was focused when the hold began: the question is asked over it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AskScope {
    /// The focused window.
    pub window: Option<WindowKey>,
    /// Its app, if it has a bus name.
    pub app: Option<AppName>,
}

/// The words heard so far in one hold.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Heard {
    /// Stable segments, never revised.
    pub committed: Words,
    /// The unstable tail after them.
    pub tail: Words,
}

impl Heard {
    /// Everything heard, committed first.
    pub fn text(&self) -> String {
        [self.committed.0.as_str(), self.tail.0.as_str()]
            .iter()
            .filter(|part| !part.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// What a command waits for once the microphone has opened: the hold may end before `Begin`
/// returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Then {
    /// Listen on.
    Listen,
    /// The key was let go.
    Release,
    /// The hold was cancelled.
    Cancel,
}

/// Where the hold stands.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Phase {
    /// Nothing is held.
    #[default]
    Idle,
    /// `Begin` was sent and has not returned.
    Opening(Then),
    /// The microphone is open.
    Live(Heard),
    /// The key was let go: the last words and the end are on their way.
    Finishing(Heard),
}

/// What the person is told when a hold did not become a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Notice {
    /// Nothing to tell.
    #[default]
    None,
    /// `voiced` refused the hold.
    Refused(VoiceRefusal),
    /// `voiced` is not on the bus.
    Unreachable,
    /// The hold ended with no words.
    NothingHeard,
    /// The utterance failed.
    Failed(VoiceFault),
}

/// Whether the first-use consent sheet is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConsentSheet {
    /// Not showing.
    #[default]
    Closed,
    /// Asking Allow or Not Now.
    Asking,
}

/// Everything the voice service publishes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Voice {
    /// Whether the bus is reached.
    pub link: VoiceLink,
    /// What the settings say.
    pub params: VoiceParams,
    /// Where the hold stands.
    pub phase: Phase,
    /// What the hold began over.
    pub scope: AskScope,
    /// What the person is told.
    pub notice: Notice,
    /// The consent sheet.
    pub sheet: ConsentSheet,
}
