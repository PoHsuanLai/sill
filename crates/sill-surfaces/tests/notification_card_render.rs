//! `NotificationCard` on its own, as a banner and as a notification center row (lane c mounts
//! it), SSR-rendered inside a `Ds`: the parts design/13 §13.3.6 lists are there, collapsed and
//! expanded; three actions become an Options menu; the center row paints the Popover material.

#[path = "support/fake.rs"]
mod fake;

use dioxus::prelude::*;
use ds::{Ds, Material};
use sill_services::{
    Action, ActionKey, AppRef, Body, Expiry, Hints, IconRef, Notification, NotificationId, UnixMs,
};
use sill_settings::{SettingsFeed, ShellFile};
use sill_surfaces::surfaces::notification_card::{
    CardPlace, CardPress, Expansion, NotificationCard,
};

const NOW: UnixMs = UnixMs(1_000_000_000);

#[derive(Props, Clone, PartialEq)]
struct Shown {
    note: Notification,
    count: usize,
    expansion: Expansion,
    place: CardPlace,
}

#[allow(non_snake_case)]
fn Root(shown: Shown) -> Element {
    rsx! {
        Ds { appearance: ds::Appearance::default(), material: Material::Toast,
            NotificationCard {
                note: shown.note,
                count: shown.count,
                expansion: shown.expansion,
                place: shown.place,
                now: NOW,
                on_press: move |_: CardPress| {},
            }
        }
    }
}

fn note(actions: &[&str]) -> Notification {
    Notification {
        id: NotificationId(4),
        arrived: UnixMs(NOW.0 - 5 * 60_000),
        app: AppRef {
            name: "Evolution".to_owned(),
            icon: IconRef::None,
            desktop_entry: None,
        },
        summary: "Lunch?".to_owned(),
        body: Body::plain("Are you free at noon on Thursday?"),
        actions: actions
            .iter()
            .map(|key| Action {
                key: ActionKey((*key).to_owned()),
                label: key.to_uppercase(),
            })
            .collect(),
        hints: Hints::default(),
        expiry: Expiry::Default,
    }
}

fn render(shown: Shown) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, _feeds) = fake::services();
    let mut dom = VirtualDom::new_with_props(Root, shown);
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(ShellFile::default()));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn a_collapsed_banner_shows_icon_app_age_summary_and_body() {
    let html = render(Shown {
        note: note(&["default", "reply"]),
        count: 1,
        expansion: Expansion::Collapsed,
        place: CardPlace::Banner,
    });
    for part in [
        "Evolution",
        ">5m<",
        "Lunch?",
        "Are you free at noon on Thursday?",
        "data-material=\"toast\"",
        "class=\"sill-card-icon\"",
    ] {
        assert!(html.contains(part), "{part} in {html}");
    }
    for absent in [
        "class=\"sill-card-close\"",
        "class=\"sill-card-actions\"",
        "class=\"ds-count\"",
        "class=\"sill-card-layer\"",
    ] {
        assert!(!html.contains(absent), "no {absent} collapsed and alone");
    }
}

#[test]
fn an_expanded_group_shows_its_count_layers_actions_and_close() {
    let html = render(Shown {
        note: note(&["default", "reply", "later"]),
        count: 3,
        expansion: Expansion::Expanded,
        place: CardPlace::Banner,
    });
    assert!(html.contains("class=\"sill-card-close\""));
    assert_eq!(html.matches("class=\"sill-card-layer\"").count(), 2);
    assert!(html.contains(">3<"), "the count");
    assert!(html.contains(">REPLY<") && html.contains(">LATER<"), "two buttons");
    assert!(!html.contains(">DEFAULT<"), "default is the body's click");
    let many = render(Shown {
        note: note(&["a", "b", "c"]),
        count: 1,
        expansion: Expansion::Expanded,
        place: CardPlace::Banner,
    });
    assert!(many.contains(">Options<"), "more than two: a menu");
}

#[test]
fn a_center_row_is_on_the_popover_material() {
    let html = render(Shown {
        note: note(&[]),
        count: 1,
        expansion: Expansion::Collapsed,
        place: CardPlace::Center,
    });
    assert!(html.contains("data-place=\"center\""));
    assert!(html.contains("data-material=\"popover\""));
}
