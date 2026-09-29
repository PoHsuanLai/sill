use sill_launcher::AppId;

use super::{Badge, BadgeEvent, BadgeProp, Badges, parse_app_uri, step};
use crate::units::{Permille, Urgency, Visibility};

fn id(s: &str) -> AppId {
    AppId(s.to_owned())
}

fn update(app: &str, props: Vec<BadgeProp>) -> BadgeEvent {
    BadgeEvent::Update {
        app: id(app),
        props,
    }
}

fn one(app: &str, badge: Badge) -> Badges {
    Badges {
        apps: [(id(app), badge)].into_iter().collect(),
    }
}

fn counting(n: u64) -> Badge {
    Badge {
        count: n,
        count_visibility: Visibility::Shown,
        ..Badge::default()
    }
}

#[test]
fn updates_merge_into_the_apps_badge() {
    let cases: Vec<(&str, Badges, BadgeEvent, Badges)> = vec![
        (
            "a visible count",
            Badges::default(),
            update(
                "mail",
                vec![
                    BadgeProp::Count(3),
                    BadgeProp::CountVisibility(Visibility::Shown),
                ],
            ),
            one("mail", counting(3)),
        ),
        (
            "a count change keeps visibility",
            one("mail", counting(3)),
            update("mail", vec![BadgeProp::Count(7)]),
            one("mail", counting(7)),
        ),
        (
            "a negative count is zero",
            one("mail", counting(3)),
            update("mail", vec![BadgeProp::Count(-4)]),
            one("mail", counting(0)),
        ),
        (
            "progress is clamped",
            Badges::default(),
            update(
                "dl",
                vec![
                    BadgeProp::Progress(Permille(1400)),
                    BadgeProp::ProgressVisibility(Visibility::Shown),
                ],
            ),
            one(
                "dl",
                Badge {
                    progress: Permille(1000),
                    progress_visibility: Visibility::Shown,
                    ..Badge::default()
                },
            ),
        ),
        (
            "urgency alone",
            Badges::default(),
            update("chat", vec![BadgeProp::Urgency(Urgency::Urgent)]),
            one(
                "chat",
                Badge {
                    urgency: Urgency::Urgent,
                    ..Badge::default()
                },
            ),
        ),
        (
            "hiding everything drops the app",
            one("mail", counting(3)),
            update(
                "mail",
                vec![
                    BadgeProp::Count(0),
                    BadgeProp::CountVisibility(Visibility::Hidden),
                ],
            ),
            Badges::default(),
        ),
        (
            "a hidden count is kept while it does not show",
            Badges::default(),
            update("mail", vec![BadgeProp::Count(5)]),
            one(
                "mail",
                Badge {
                    count: 5,
                    ..Badge::default()
                },
            ),
        ),
        (
            "cleared",
            one("mail", counting(3)),
            BadgeEvent::Cleared(id("mail")),
            Badges::default(),
        ),
        (
            "cleared all",
            one("mail", counting(3)),
            BadgeEvent::ClearedAll,
            Badges::default(),
        ),
    ];
    for (name, before, event, want) in cases {
        assert_eq!(step(before, event), want, "{name}");
    }
}

#[test]
fn shown_values_need_visibility() {
    assert_eq!(counting(3).shown_count(), Some(3));
    assert_eq!(counting(0).shown_count(), None);
    let hidden = Badge {
        count: 3,
        ..Badge::default()
    };
    assert_eq!(hidden.shown_count(), None);
    assert_eq!(hidden.shown_progress(), None);
}

#[test]
fn app_uris_name_desktop_ids() {
    const CASES: &[(&str, Option<&str>)] = &[
        (
            "application://org.gnome.Nautilus.desktop",
            Some("org.gnome.Nautilus"),
        ),
        ("application://firefox.desktop", Some("firefox")),
        ("application://firefox", Some("firefox")),
        ("application://.desktop", None),
        ("application://", None),
        ("file:///usr/share/applications/firefox.desktop", None),
        ("application://../../etc/passwd", None),
        ("", None),
    ];
    for &(uri, want) in CASES {
        assert_eq!(parse_app_uri(uri), want.map(id), "{uri}");
    }
}
