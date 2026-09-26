//! The one place the clipboard history changes. Pure; lane b fills it (FINDINGS "M9 freeze"
//! F647), its rules pinned by the tables below:
//!
//! - `Command(Policy)`: kept as the state's policy and published as `status.mode`; turning
//!   history on from off is `Connect`, turning it off is `Disconnect` and drops every entry
//!   (the history is gone, not hidden); a smaller cap drops the oldest.
//! - `Connected(p)` publishes `p`; `Disconnected` publishes `NotConnected` (entries stay).
//! - `Copied`: ignored while `Off`; otherwise the newest entry, with the next key. The same
//!   content already kept moves to the top under its old key rather than being added twice
//!   (so offering an entry again, which the compositor reports back as a new selection, does
//!   not duplicate it). Past the cap the oldest goes.
//! - `Skipped(_)` changes nothing (nothing about a skipped selection is kept).
//! - `Command(Offer(key))`: `OfferSelection(entry)` for a kept key, nothing otherwise.
//! - `Command(Forget(key))` drops it; `Command(Clear)` drops all.

use super::model::{ClipEffect, ClipEvent, ClipState};

/// `state` after `event`.
pub fn reduce_clipboard(state: ClipState, event: ClipEvent) -> (ClipState, Vec<ClipEffect>) {
    let _ = (state, event);
    todo!("M9 fill: b-clipboard")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sill_launcher::{ClipKey, UnixTime};

    use super::reduce_clipboard;
    use crate::clipboard::model::{
        ClipContent, ClipEffect, ClipEvent, ClipPolicy, ClipProtocol, ClipState, ClipText,
        ClipboardCommand, EntryCap, HistoryMode, ImageCap, Skipped,
    };

    fn text(s: &str) -> ClipContent {
        ClipContent::Text(ClipText(Arc::from(s)))
    }

    fn on(cap: u16) -> ClipEvent {
        ClipEvent::Command(ClipboardCommand::Policy(ClipPolicy {
            mode: HistoryMode::Memory,
            cap: EntryCap(cap),
            images: ImageCap(8 << 20),
        }))
    }

    fn copied(s: &str, at: u64) -> ClipEvent {
        ClipEvent::Copied {
            content: text(s),
            at: UnixTime(at),
        }
    }

    fn run(events: Vec<ClipEvent>) -> (ClipState, Vec<ClipEffect>) {
        events
            .into_iter()
            .fold((ClipState::default(), Vec::new()), |(state, _), event| {
                reduce_clipboard(state, event)
            })
    }

    fn keys(state: &ClipState) -> Vec<u64> {
        state.shown.entries.iter().map(|e| e.key.0).collect()
    }

    #[test]
    #[ignore = "M9 fill: b-clipboard"]
    fn nothing_is_kept_until_history_is_on() {
        let (state, _) = run(vec![copied("a", 1)]);
        assert!(state.shown.entries.is_empty());
        assert_eq!(state.shown.status.mode, HistoryMode::Off);
        let (_, effects) = run(vec![on(5)]);
        assert_eq!(effects, vec![ClipEffect::Connect]);
    }

    #[test]
    #[ignore = "M9 fill: b-clipboard"]
    fn entries_are_newest_first_capped_and_never_duplicated() {
        let (state, _) = run(vec![
            on(3),
            copied("a", 1),
            copied("b", 2),
            copied("c", 3),
            copied("d", 4),
        ]);
        assert_eq!(keys(&state), [4, 3, 2], "the oldest went past the cap");
        let (state, _) = run(vec![on(3), copied("a", 1), copied("b", 2), copied("a", 3)]);
        assert_eq!(keys(&state), [1, 2], "a moved to the top under its key");
        assert_eq!(state.shown.entries[0].copied, UnixTime(3));
    }

    #[test]
    #[ignore = "M9 fill: b-clipboard"]
    fn turning_history_off_forgets_everything_and_disconnects() {
        let (state, effects) = run(vec![
            on(5),
            copied("a", 1),
            ClipEvent::Command(ClipboardCommand::Policy(ClipPolicy::default())),
        ]);
        assert!(state.shown.entries.is_empty());
        assert_eq!(effects, vec![ClipEffect::Disconnect]);
    }

    #[test]
    #[ignore = "M9 fill: b-clipboard"]
    fn offer_forget_clear_and_skips() {
        let (state, effects) = run(vec![
            on(5),
            copied("a", 1),
            ClipEvent::Command(ClipboardCommand::Offer(ClipKey(1))),
        ]);
        assert_eq!(
            effects,
            vec![ClipEffect::OfferSelection(state.shown.entries[0].clone())]
        );
        let (_, effects) = run(vec![
            on(5),
            ClipEvent::Command(ClipboardCommand::Offer(ClipKey(9))),
        ]);
        assert!(effects.is_empty(), "an unknown key offers nothing");
        let (state, _) = run(vec![
            on(5),
            copied("a", 1),
            copied("b", 2),
            ClipEvent::Command(ClipboardCommand::Forget(ClipKey(1))),
        ]);
        assert_eq!(keys(&state), [2]);
        let (state, _) = run(vec![
            on(5),
            copied("a", 1),
            ClipEvent::Skipped(Skipped::Secret),
            ClipEvent::Command(ClipboardCommand::Clear),
        ]);
        assert!(state.shown.entries.is_empty());
        let (state, _) = run(vec![on(5), ClipEvent::Connected(ClipProtocol::Ext)]);
        assert_eq!(state.shown.status.protocol, ClipProtocol::Ext);
    }
}
