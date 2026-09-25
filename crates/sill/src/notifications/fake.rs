//! `sill debug notify`: a made-up notification as the posting a `Notify` would have made.
//!
//! Lane a (FINDINGS "The M6 fill-in") parses the body with `sill_services::parse_body` once it is
//! filled, as the D-Bus path does; until then the body is plain text (calling the frozen stub
//! would panic the daemon).

use sill_ipc::{FakeExpire, FakeNotification, FakeUrgency};
use sill_services::{
    Action, ActionKey, AppRef, Body, Expiry, Hints, IconRef, NotificationId, NotifyUrgency, Posting,
};

/// The posting `fake` stands for.
pub fn posting_of(fake: FakeNotification) -> Posting {
    let FakeNotification {
        app,
        summary,
        body,
        urgency,
        actions,
        expire,
        replaces,
    } = fake;
    Posting {
        replaces: NotificationId::replacing(replaces),
        app: AppRef {
            name: app,
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary,
        body: Body::plain(&body),
        actions: actions
            .into_iter()
            .map(|action| Action {
                key: ActionKey(action.key),
                label: action.label,
            })
            .collect(),
        hints: Hints {
            urgency: match urgency {
                FakeUrgency::Low => NotifyUrgency::Low,
                FakeUrgency::Normal => NotifyUrgency::Normal,
                FakeUrgency::Critical => NotifyUrgency::Critical,
            },
            ..Hints::default()
        },
        expiry: match expire {
            FakeExpire::Default => Expiry::Default,
            FakeExpire::Never => Expiry::Never,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::posting_of;
    use sill_ipc::{FakeAction, FakeExpire, FakeNotification, FakeUrgency};
    use sill_services::{ActionKey, AppKey, Expiry, NotificationId, NotifyUrgency};

    #[test]
    fn a_fake_becomes_the_posting_notify_would_make() {
        let posting = posting_of(FakeNotification {
            app: "Mail".to_owned(),
            summary: "Ada".to_owned(),
            body: "the engine".to_owned(),
            urgency: FakeUrgency::Critical,
            actions: vec![FakeAction {
                key: "default".to_owned(),
                label: "Open".to_owned(),
            }],
            expire: FakeExpire::Never,
            replaces: 3,
        });
        assert_eq!(posting.replaces, Some(NotificationId(3)));
        assert_eq!(posting.app.key(), AppKey("Mail".to_owned()));
        assert_eq!(posting.summary, "Ada");
        assert_eq!(posting.body.text(), "the engine");
        assert_eq!(posting.actions[0].key, ActionKey::default_action());
        assert_eq!(posting.hints.urgency, NotifyUrgency::Critical);
        assert_eq!(posting.expiry, Expiry::Never);
    }
}
