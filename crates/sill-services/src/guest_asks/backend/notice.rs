//! The notification about a computer asking: its posting.

use sill_model::guest_asks::{Ask, NOTICE_ENTRY, Verdict, choice_key, notice_text};
use sill_model::notifications::{
    Action, AppRef, Body, Expiry, Hints, IconRef, NotificationId, NotifyUrgency, Posting,
};

/// The posting that asks the person `ask`, in place of `replaces` when one is shown. It stays
/// until withdrawn or closed, and carries Allow and Deny.
pub(crate) fn posting(ask: &Ask, replaces: Option<NotificationId>) -> Posting {
    let notice = notice_text(ask);
    Posting {
        replaces,
        app: AppRef {
            name: "Computers".to_owned(),
            icon: IconRef::Name("computer-symbolic".to_owned()),
            desktop_entry: Some(NOTICE_ENTRY.to_owned()),
        },
        summary: notice.summary,
        body: Body::plain(&notice.body),
        actions: vec![
            Action {
                key: choice_key(&ask.node, Verdict::Allow),
                label: notice.allow,
            },
            Action {
                key: choice_key(&ask.node, Verdict::Deny),
                label: notice.deny,
            },
        ],
        hints: Hints {
            urgency: NotifyUrgency::Normal,
            ..Hints::default()
        },
        expiry: Expiry::Never,
    }
}
