//! `org.quire.Voice1` and its utterance objects, from voiced's interface description. Bodies are
//! JSON in `s`; the event stream is an fd.

use zbus::zvariant::{OwnedFd, OwnedObjectPath};

/// The bus name voiced serves.
pub(super) const VOICE_BUS: &str = "org.quire.Voice1";
/// Where utterance objects live, `<prefix>/<n>`.
pub(super) const UTTERANCE_PREFIX: &str = "/org/quire/Voice1/utterance";

/// The error voiced answers a body that does not parse with (`VoiceError::Malformed`), and a
/// `Cancel` whose cause it will not take (superseded, too long, empty, bad JSON): the microphone
/// stays open.
pub(super) const MALFORMED: &str = "org.quire.Voice1.Error.Malformed";

/// The caller's side of `org.quire.Voice1`.
#[zbus::proxy(
    interface = "org.quire.Voice1",
    default_service = "org.quire.Voice1",
    default_path = "/org/quire/Voice1"
)]
pub(super) trait Voice {
    /// Opens the microphone now (shell only): `VoiceBegin` in, the utterance object and an event
    /// stream out.
    fn begin(&self, begin: &str) -> zbus::Result<(OwnedObjectPath, OwnedFd)>;
}

/// The caller's side of one utterance object.
#[zbus::proxy(
    interface = "org.quire.Voice1.Utterance",
    default_service = "org.quire.Voice1"
)]
pub(super) trait Utterance {
    /// The end of the hold: finish and send.
    fn release(&self) -> zbus::Result<()>;
    /// Discards everything, for `cause`: a sealed `CancelCause`.
    fn cancel(&self, cause: &str) -> zbus::Result<()>;
}
