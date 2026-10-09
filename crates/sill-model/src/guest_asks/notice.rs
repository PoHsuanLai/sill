//! Telling the person: the words, the two buttons, and finding the notification already shown.

use crate::notifications::{ActionKey, NotesState, NotificationId};

use super::model::{Ask, Computer, Verdict};

/// The desktop entry the notifications carry; it names them among the notifications held.
pub const NOTICE_ENTRY: &str = "sill-guest-asks";

/// What an Allow button's key says before the computer.
const ALLOW_PREFIX: &str = "allow-guest:";
/// What a Deny button's key says before the computer.
const DENY_PREFIX: &str = "deny-guest:";

/// A notification's words and the labels of its two buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub summary: String,
    pub body: String,
    pub allow: String,
    pub deny: String,
}

/// What the person is asked about `ask`.
pub fn notice_text(ask: &Ask) -> Notice {
    Notice {
        summary: format!("Let {} use this computer's models?", ask.name),
        body: "It is one of your computers.".to_owned(),
        allow: "Allow".to_owned(),
        deny: "Deny".to_owned(),
    }
}

/// The action key of the button that gives `node` the `verdict`.
pub fn choice_key(node: &Computer, verdict: Verdict) -> ActionKey {
    let prefix = match verdict {
        Verdict::Allow => ALLOW_PREFIX,
        Verdict::Deny => DENY_PREFIX,
    };
    ActionKey(format!("{prefix}{}", node.as_str()))
}

/// The computer and verdict a button's action key stands for; `None` for any other key.
pub fn choice_of(key: &ActionKey) -> Option<(Computer, Verdict)> {
    [(ALLOW_PREFIX, Verdict::Allow), (DENY_PREFIX, Verdict::Deny)]
        .into_iter()
        .find_map(|(prefix, verdict)| {
            key.0
                .strip_prefix(prefix)
                .filter(|node| !node.is_empty())
                .map(|node| (Computer::new(node), verdict))
        })
}

/// The notification for `node` already held, banner or history.
pub fn shown_for(notes: &NotesState, node: &Computer) -> Option<NotificationId> {
    let key = choice_key(node, Verdict::Allow);
    let banners = notes.live.banners.iter().map(|banner| &banner.note);
    let kept = notes.history.groups.iter().flat_map(|group| &group.notes);
    banners
        .chain(kept)
        .filter(|note| note.app.desktop_entry.as_deref() == Some(NOTICE_ENTRY))
        .find(|note| note.actions.iter().any(|action| action.key == key))
        .map(|note| note.id)
}
