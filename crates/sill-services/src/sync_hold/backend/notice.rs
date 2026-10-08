//! The notification about a held deletion: its posting.

use sill_model::notifications::{
    Action, AppRef, Body, Expiry, Hints, IconRef, NotificationId, NotifyUrgency, Posting,
};
use sill_model::sync_hold::{Hold, NOTICE_ENTRY, notice_text, open_key};

/// The posting that tells the person `hold`, in place of `replaces` when one is shown. It stays
/// until withdrawn or closed, and carries the one button.
pub(crate) fn posting(hold: &Hold, replaces: Option<NotificationId>) -> Posting {
    let notice = notice_text(hold);
    Posting {
        replaces,
        app: AppRef {
            name: "Files".to_owned(),
            icon: IconRef::Name("folder-symbolic".to_owned()),
            desktop_entry: Some(NOTICE_ENTRY.to_owned()),
        },
        summary: notice.summary,
        body: Body::plain(&notice.body),
        actions: vec![Action {
            key: open_key(&hold.dataset),
            label: notice.button,
        }],
        hints: Hints {
            urgency: NotifyUrgency::Normal,
            ..Hints::default()
        },
        expiry: Expiry::Never,
    }
}
