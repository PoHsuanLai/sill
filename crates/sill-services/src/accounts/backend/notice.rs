//! The notification about an account: its posting.

use sill_model::accounts::{NOTICE_ENTRY, NeedsSignIn, notice_text, sign_in_key};
use sill_model::notifications::{
    Action, AppRef, Body, Expiry, Hints, IconRef, NotificationId, NotifyUrgency, Posting,
};

/// The posting that tells the person `needing`, in place of `replaces` when one is shown. It
/// stays until acted on or withdrawn, and carries the one button.
pub(crate) fn posting(needing: &NeedsSignIn, replaces: Option<NotificationId>) -> Posting {
    let notice = notice_text(needing);
    Posting {
        replaces,
        app: AppRef {
            name: "Accounts".to_owned(),
            icon: IconRef::Name("avatar-default".to_owned()),
            desktop_entry: Some(NOTICE_ENTRY.to_owned()),
        },
        summary: notice.summary,
        body: Body::plain(&notice.body),
        actions: vec![Action {
            key: sign_in_key(&needing.account),
            label: notice.button,
        }],
        hints: Hints {
            urgency: NotifyUrgency::Normal,
            ..Hints::default()
        },
        expiry: Expiry::Never,
    }
}
