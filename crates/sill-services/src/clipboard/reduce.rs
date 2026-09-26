//! The one place the clipboard history changes. Pure (FINDINGS "M9 freeze" F647, "M9 lane b"
//! F686), its rules pinned by the tables below:
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

use sill_launcher::{ClipKey, UnixTime};

use super::model::{
    ClipContent, ClipEffect, ClipEntry, ClipEvent, ClipPolicy, ClipProtocol, ClipState,
    ClipboardCommand, EntryCap, HistoryMode, ImageCap,
};

/// `state` after `event`.
pub fn reduce_clipboard(state: ClipState, event: ClipEvent) -> (ClipState, Vec<ClipEffect>) {
    match event {
        ClipEvent::Command(ClipboardCommand::Policy(policy)) => with_policy(state, policy),
        ClipEvent::Command(ClipboardCommand::Offer(key)) => {
            let effects = entry(&state, key)
                .map(|found| ClipEffect::OfferSelection(found.clone()))
                .into_iter()
                .collect();
            (state, effects)
        }
        ClipEvent::Command(ClipboardCommand::Forget(key)) => {
            (retain(state, |kept| kept.key != key), Vec::new())
        }
        ClipEvent::Command(ClipboardCommand::Clear) => (retain(state, |_| false), Vec::new()),
        ClipEvent::Connected(protocol) => (with_protocol(state, protocol), Vec::new()),
        ClipEvent::Disconnected => (with_protocol(state, ClipProtocol::NotConnected), Vec::new()),
        ClipEvent::Copied { content, at } => (copied(state, content, at), Vec::new()),
        ClipEvent::Skipped(_) => (state, Vec::new()),
    }
}

/// The new policy kept and published; on from off connects, off forgets and disconnects, and
/// a smaller cap (entries or image size) drops what no longer fits.
fn with_policy(mut state: ClipState, policy: ClipPolicy) -> (ClipState, Vec<ClipEffect>) {
    let was = state.policy.mode;
    state.policy = policy;
    state.shown.status.mode = policy.mode;
    match (was, policy.mode) {
        (HistoryMode::Off, HistoryMode::Memory) => (fitted(state), vec![ClipEffect::Connect]),
        (HistoryMode::Memory, HistoryMode::Off) => {
            state.shown.entries.clear();
            state.shown.status.protocol = ClipProtocol::NotConnected;
            (state, vec![ClipEffect::Disconnect])
        }
        (HistoryMode::Memory, HistoryMode::Memory) => (fitted(state), Vec::new()),
        (HistoryMode::Off, HistoryMode::Off) => (state, Vec::new()),
    }
}

/// The protocol published, unless history is off (a connection's last word arriving after it
/// was dropped says nothing true).
fn with_protocol(mut state: ClipState, protocol: ClipProtocol) -> ClipState {
    state.shown.status.protocol = match state.policy.mode {
        HistoryMode::Memory => protocol,
        HistoryMode::Off => ClipProtocol::NotConnected,
    };
    state
}

/// `content` as the newest entry: under its old key when the same content is already kept,
/// else under the next key; nothing while off or when the policy does not keep it.
fn copied(mut state: ClipState, content: ClipContent, at: UnixTime) -> ClipState {
    if state.policy.mode == HistoryMode::Off || !fits(&content, state.policy.images) {
        return state;
    }
    let old = state
        .shown
        .entries
        .iter()
        .position(|kept| kept.content == content)
        .map(|at| state.shown.entries.remove(at).key);
    let key = old.unwrap_or_else(|| {
        let key = ClipKey(state.next.max(1));
        state.next = key.0 + 1;
        key
    });
    state.shown.entries.insert(
        0,
        ClipEntry {
            key,
            content,
            copied: at,
        },
    );
    fitted(state)
}

/// At most the cap's entries (the oldest go first), and no image over the image cap.
fn fitted(state: ClipState) -> ClipState {
    let EntryCap(cap) = state.policy.cap;
    let images = state.policy.images;
    let mut state = retain(state, |kept| fits(&kept.content, images));
    state.shown.entries.truncate(usize::from(cap));
    state
}

/// Whether the policy keeps `content`: text always, an image up to the cap (zero keeps none).
fn fits(content: &ClipContent, ImageCap(images): ImageCap) -> bool {
    match content {
        ClipContent::Text(_) => true,
        ClipContent::Image(image) => {
            images > 0 && u64::try_from(image.bytes.len()).is_ok_and(|len| len <= images)
        }
    }
}

fn retain(mut state: ClipState, keep: impl FnMut(&ClipEntry) -> bool) -> ClipState {
    state.shown.entries.retain(keep);
    state
}

fn entry(state: &ClipState, key: ClipKey) -> Option<&ClipEntry> {
    state.shown.entries.iter().find(|kept| kept.key == key)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sill_launcher::{ClipKey, Mime, UnixTime};

    use super::reduce_clipboard;
    use crate::clipboard::model::{
        ClipContent, ClipEffect, ClipEvent, ClipImage, ClipPolicy, ClipProtocol, ClipState,
        ClipText, ClipboardCommand, EntryCap, HistoryMode, ImageCap, Skipped,
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
    fn nothing_is_kept_until_history_is_on() {
        let (state, _) = run(vec![copied("a", 1)]);
        assert!(state.shown.entries.is_empty());
        assert_eq!(state.shown.status.mode, HistoryMode::Off);
        let (_, effects) = run(vec![on(5)]);
        assert_eq!(effects, vec![ClipEffect::Connect]);
    }

    #[test]
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

    fn image(len: usize) -> ClipContent {
        ClipContent::Image(ClipImage {
            mime: Mime("image/png".to_owned()),
            bytes: Arc::from(vec![0_u8; len]),
            width: 1,
            height: 1,
        })
    }

    fn policy(mode: HistoryMode, cap: u16, images: u64) -> ClipEvent {
        ClipEvent::Command(ClipboardCommand::Policy(ClipPolicy {
            mode,
            cap: EntryCap(cap),
            images: ImageCap(images),
        }))
    }

    #[test]
    fn a_smaller_policy_drops_what_no_longer_fits() {
        let img = |len, at| ClipEvent::Copied {
            content: image(len),
            at: UnixTime(at),
        };
        let cases: Vec<(Vec<ClipEvent>, Vec<u64>)> = vec![
            (
                vec![on(5), copied("a", 1), copied("b", 2), copied("c", 3), on(2)],
                vec![3, 2],
            ),
            (vec![policy(HistoryMode::Memory, 5, 0), img(4, 1)], vec![]),
            (vec![policy(HistoryMode::Memory, 5, 3), img(4, 1)], vec![]),
            (
                vec![policy(HistoryMode::Memory, 5, 4), img(4, 1), copied("a", 2)],
                vec![2, 1],
            ),
            (
                vec![
                    policy(HistoryMode::Memory, 5, 8),
                    img(8, 1),
                    copied("a", 2),
                    policy(HistoryMode::Memory, 5, 4),
                ],
                vec![2],
            ),
            (vec![on(5), copied("a", 1), on(0)], vec![]),
        ];
        for (events, want) in cases {
            let (state, effects) = run(events);
            assert_eq!(keys(&state), want);
            assert!(effects.is_empty(), "a policy that stays on connects once");
        }
    }

    #[test]
    fn keys_are_never_reused_and_the_protocol_follows_the_connection() {
        let (state, _) = run(vec![
            on(5),
            copied("a", 1),
            ClipEvent::Command(ClipboardCommand::Clear),
            copied("a", 2),
        ]);
        assert_eq!(
            keys(&state),
            [2],
            "a cleared entry's key is not given again"
        );
        let cases: Vec<(Vec<ClipEvent>, ClipProtocol)> = vec![
            (
                vec![ClipEvent::Connected(ClipProtocol::Ext)],
                ClipProtocol::NotConnected,
            ),
            (
                vec![on(5), ClipEvent::Connected(ClipProtocol::Wlr)],
                ClipProtocol::Wlr,
            ),
            (
                vec![
                    on(5),
                    ClipEvent::Connected(ClipProtocol::Ext),
                    ClipEvent::Disconnected,
                ],
                ClipProtocol::NotConnected,
            ),
            (
                vec![
                    on(5),
                    ClipEvent::Connected(ClipProtocol::Unsupported),
                    policy(HistoryMode::Off, 5, 0),
                ],
                ClipProtocol::NotConnected,
            ),
        ];
        for (events, want) in cases {
            assert_eq!(run(events).0.shown.status.protocol, want);
        }
    }
}
