//! The notification about keycap: its posting, and finding the one already shown.

use sill_model::keycap::{Attention, notice_text};
use sill_model::notifications::{
    AppKey, AppRef, Body, Expiry, Hints, IconRef, NotesState, NotificationId, Posting,
};

/// The desktop entry the notices carry; it names them among the notifications held.
const ENTRY: &str = "sill-keycap";

/// The posting that tells the person `attention`, in place of `replaces` when one is shown.
pub(crate) fn posting(attention: &Attention, replaces: Option<NotificationId>) -> Posting {
    let notice = notice_text(attention);
    Posting {
        replaces,
        app: AppRef {
            name: "Keyboard".to_owned(),
            icon: IconRef::Name("input-keyboard".to_owned()),
            desktop_entry: Some(ENTRY.to_owned()),
        },
        summary: notice.summary,
        body: Body::plain(&notice.body),
        actions: Vec::new(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    }
}

/// The notice already held, banner or history.
pub(crate) fn shown(notes: &NotesState) -> Option<NotificationId> {
    let key = AppKey(ENTRY.to_owned());
    notes
        .live
        .banners
        .iter()
        .map(|banner| &banner.note)
        .find(|note| note.app.key() == key)
        .or_else(|| {
            notes
                .history
                .group(&key)
                .and_then(|group| group.notes.first())
        })
        .map(|note| note.id)
}
