//! Notices a service posts through the notifications service and answers when their buttons are
//! pressed: which notification id carries which key, so a press is answered only when the id is
//! one sill itself posted for that key. The accounts, sync-hold and guest-asks services each keep
//! one of these over their own key (an account, a dataset, a computer); they differ only in the
//! words and in how an action key names its notice.

use std::collections::HashMap;
use std::hash::Hash;

use super::{ActionKey, NotesState, NotificationId};

/// The id of the notification posted for each key, kept from when it was seen in the notes until
/// the server says it closed or the key's notice is withdrawn. It outlives the notes' own copy on
/// purpose: a pressed button leaves the notes before its `ActionInvoked` is heard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notices<K: Eq + Hash>(HashMap<K, NotificationId>);

impl<K: Eq + Hash> Default for Notices<K> {
    fn default() -> Self {
        Notices(HashMap::new())
    }
}

impl<K: Eq + Hash + Clone> Notices<K> {
    /// Remember the notification the notes hold for each of `keys`, as `shown_for` finds it; one
    /// posted afresh replaces the id kept for its key.
    pub fn learn<'a>(
        &mut self,
        notes: &NotesState,
        keys: impl IntoIterator<Item = &'a K>,
        shown_for: impl Fn(&NotesState, &K) -> Option<NotificationId>,
    ) where
        K: 'a,
    {
        for key in keys {
            if let Some(id) = shown_for(notes, key) {
                self.0.insert(key.clone(), id);
            }
        }
    }

    /// The server closed `id`: it answers nothing from now on.
    pub fn closed(&mut self, id: NotificationId) {
        self.0.retain(|_, kept| *kept != id);
    }

    /// The key's notice ended, or is being replaced: its id is no longer one.
    pub fn forget(&mut self, key: &K) {
        self.0.remove(key);
    }

    /// What a press answers: `pressed`, which a service reads from the button's action key, and
    /// only when `id` is the notification posted for the key it names (`key_of`). Any other id,
    /// or the right one with another key's button, answers nothing.
    pub fn answers<R>(
        &self,
        id: NotificationId,
        pressed: Option<R>,
        key_of: impl Fn(&R) -> &K,
    ) -> Option<R> {
        pressed.filter(|pressed| self.0.get(key_of(pressed)) == Some(&id))
    }
}

/// The notification already held, banner or history, that belongs to the app named `entry` and
/// carries a button with the action `key`.
pub fn shown_with_action(
    notes: &NotesState,
    entry: &str,
    key: &ActionKey,
) -> Option<NotificationId> {
    let banners = notes.live.banners.iter().map(|banner| &banner.note);
    let kept = notes.history.groups.iter().flat_map(|group| &group.notes);
    banners
        .chain(kept)
        .filter(|note| note.app.desktop_entry.as_deref() == Some(entry))
        .find(|note| note.actions.iter().any(|action| action.key == *key))
        .map(|note| note.id)
}

#[cfg(test)]
mod tests {
    use super::{Notices, shown_with_action};
    use crate::notifications::{
        Action, ActionKey, AppGroup, AppKey, AppRef, BannerLife, Body, Expiry, Hints, IconRef,
        LiveBanner, NotesState, Notification, NotificationId, UnixMs,
    };

    const ENTRY: &str = "sill-test";

    fn key_of(name: &str) -> ActionKey {
        ActionKey(format!("press:{name}"))
    }

    fn name_of(key: &ActionKey) -> Option<String> {
        key.0.strip_prefix("press:").map(str::to_owned)
    }

    fn shown_for(notes: &NotesState, name: &String) -> Option<NotificationId> {
        shown_with_action(notes, ENTRY, &key_of(name))
    }

    fn note(id: u32, entry: &str, name: &str) -> Notification {
        Notification {
            id: NotificationId(id),
            arrived: UnixMs(0),
            app: AppRef {
                name: String::new(),
                icon: IconRef::None,
                desktop_entry: Some(entry.to_owned()),
            },
            summary: String::new(),
            body: Body::plain(""),
            actions: vec![Action {
                key: key_of(name),
                label: String::new(),
            }],
            hints: Hints::default(),
            expiry: Expiry::Default,
        }
    }

    fn banner(note: Notification) -> LiveBanner {
        LiveBanner {
            note,
            life: BannerLife::default(),
        }
    }

    /// What a press of `key` answers with id `id`, reading the key as the services do.
    fn answer(notices: &Notices<String>, id: u32, key: &ActionKey) -> Option<String> {
        notices.answers(NotificationId(id), name_of(key), |name| name)
    }

    #[test]
    fn the_notice_shown_is_found_by_its_key_among_banners_and_history() {
        let (a, b) = ("a".to_owned(), "b".to_owned());
        let mut notes = NotesState::default();
        notes.live.banners = vec![
            banner(note(1, ENTRY, "a")),
            // Another app's action that happens to carry the same key is not ours.
            banner(note(2, "other", "b")),
        ];
        assert_eq!(shown_for(&notes, &a), Some(NotificationId(1)));
        assert_eq!(shown_for(&notes, &b), None);

        notes.live.banners.clear();
        notes.history.groups = vec![AppGroup {
            key: AppKey(ENTRY.to_owned()),
            notes: vec![note(7, ENTRY, "b")],
        }];
        assert_eq!(shown_for(&notes, &b), Some(NotificationId(7)));
        assert_eq!(shown_for(&notes, &a), None);
    }

    #[test]
    fn a_press_answers_only_for_the_id_posted_for_that_key() {
        let (a, b) = ("a".to_owned(), "b".to_owned());
        let mut notes = NotesState::default();
        notes.live.banners = vec![banner(note(1, ENTRY, "a")), banner(note(2, ENTRY, "b"))];
        let mut notices = Notices::default();
        notices.learn(&notes, [&a, &b], shown_for);
        assert_eq!(answer(&notices, 1, &key_of("a")), Some(a.clone()));
        assert_eq!(answer(&notices, 2, &key_of("b")), Some(b.clone()));
        // The right id with the wrong key, a foreign id, and a key that is not a press.
        assert_eq!(answer(&notices, 1, &key_of("b")), None);
        assert_eq!(answer(&notices, 9, &key_of("a")), None);
        assert_eq!(answer(&notices, 1, &ActionKey("default".to_owned())), None);

        // A pressed button leaves the notes before its signal is heard; the id still answers.
        notes.live.banners.clear();
        notices.learn(&notes, [&a, &b], shown_for);
        assert_eq!(answer(&notices, 1, &key_of("a")), Some(a.clone()));
        // Closed by the server: it answers nothing, and the other key's is untouched.
        notices.closed(NotificationId(1));
        assert_eq!(answer(&notices, 1, &key_of("a")), None);
        assert_eq!(answer(&notices, 2, &key_of("b")), Some(b.clone()));
        // A key whose notice ended forgets its id.
        notices.forget(&b);
        assert_eq!(answer(&notices, 2, &key_of("b")), None);
    }

    #[test]
    fn a_fresh_post_replaces_the_id_kept() {
        let a = "a".to_owned();
        let mut notes = NotesState::default();
        notes.live.banners = vec![banner(note(1, ENTRY, "a"))];
        let mut notices = Notices::default();
        notices.learn(&notes, [&a], shown_for);
        notes.live.banners = vec![banner(note(5, ENTRY, "a"))];
        notices.learn(&notes, [&a], shown_for);
        assert_eq!(answer(&notices, 1, &key_of("a")), None);
        assert_eq!(answer(&notices, 5, &key_of("a")), Some(a));
    }
}
