use super::{
    AppGroup, AppKey, AppRef, Body, Expiry, Hints, History, IconRef, Notification, NotificationId,
    UnixMs,
};

fn note(id: u32) -> Notification {
    Notification {
        id: NotificationId(id),
        arrived: UnixMs(0),
        app: AppRef {
            name: "app".to_owned(),
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary: String::new(),
        body: Body::default(),
        actions: Vec::new(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    }
}

#[test]
fn the_history_counts_and_finds_across_groups() {
    let history = History {
        groups: vec![
            AppGroup {
                key: AppKey("a".to_owned()),
                notes: vec![note(3), note(1)],
            },
            AppGroup {
                key: AppKey("b".to_owned()),
                notes: vec![note(2)],
            },
        ],
    };
    assert_eq!(history.len(), 3);
    assert!(!history.is_empty());
    assert_eq!(history.find(NotificationId(2)).map(|n| n.id.0), Some(2));
    assert_eq!(history.find(NotificationId(9)), None);
    assert!(history.group(&AppKey("b".to_owned())).is_some());
    assert!(History::default().is_empty());
}
