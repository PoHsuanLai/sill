//! What the clipboard service hears and what it can be told.

use sill_launcher::{ClipKey, UnixTime};

use super::{ClipContent, ClipEntry, ClipPolicy, ClipProtocol};

/// What the service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardCommand {
    /// The settings changed (the daemon sends it at start; the service is passive until then).
    Policy(ClipPolicy),
    /// Make this entry the clipboard (and the primary selection's owner does not change).
    Offer(ClipKey),
    /// Drop one entry.
    Forget(ClipKey),
    /// Drop every entry (`sill clipboard clear`).
    Clear,
}

/// Why an offered selection was not kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Skipped {
    /// A password manager marked it (`x-kde-passwordManagerHint` and its kin).
    Secret,
    /// No text or image type the service reads.
    NoKnownType,
    /// Larger than the policy allows.
    TooLarge,
    /// Empty, or not valid UTF-8 text.
    Empty,
}

/// What happened, for [`step`](super::step).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipEvent {
    /// The connection is up with this protocol (or none).
    Connected(ClipProtocol),
    /// The connection ended.
    Disconnected,
    /// A new selection was read whole.
    Copied { content: ClipContent, at: UnixTime },
    /// A new selection was not kept.
    Skipped(Skipped),
    /// A command arrived.
    Command(ClipboardCommand),
}

/// What the live loop must do after a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipEffect {
    /// Bind the protocol and follow the selection.
    Connect,
    /// Drop the connection (history turned off).
    Disconnect,
    /// Offer this entry's content as the selection.
    OfferSelection(ClipEntry),
}
