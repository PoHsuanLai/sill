//! `NoteCard` on its own, as a banner and as a notification center row, SSR-rendered inside a
//! `Ds`: it is quire's `NotificationCard` with sill's words: the app's name, the age, the
//! summary, the body as quire's runs, every action but `default` as a button, a group's count
//! and layers, the plate's id; a center row paints the Popover material, and an inverse banner
//! the other scheme.

use sill_ui_kit::harness::fake;

use dioxus::prelude::*;
use ds::prelude::*;
use sill_model::notifications::{
    Action, ActionKey, AppRef, Expiry, Hints, IconRef, Notification, NotificationId, UnixMs,
    parse_body,
};
use sill_settings::ShellFile;
use sill_ui_kit::notification_card::{CardPlace, CardPress, NoteCard};
use sill_ui_kit::settings::SettingsFeed;

const NOW: UnixMs = UnixMs(1_000_000_000);

#[derive(Props, Clone, PartialEq)]
struct Shown {
    note: Notification,
    count: usize,
    place: CardPlace,
}

#[allow(non_snake_case)]
fn Root(shown: Shown) -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Toast,
            NoteCard {
                note: shown.note,
                count: shown.count,
                place: shown.place,
                now: NOW,
                id: "toast-4".to_owned(),
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
        body: parse_body("Are you free at <b>noon</b> on <i>Thursday</i>?"),
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

fn render_with(shown: Shown, file: ShellFile) -> String {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("a runtime builds"); // test-only
    let _entered = rt.enter();
    let (services, _feeds) = fake::services();
    let mut dom = VirtualDom::new_with_props(Root, shown);
    dom.provide_root_context(services);
    dom.provide_root_context(SettingsFeed::fixed(file));
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn render(shown: Shown) -> String {
    render_with(shown, ShellFile::default())
}

#[test]
fn a_banner_is_quires_card_with_sills_words() {
    let html = render(Shown {
        note: note(&["default", "reply"]),
        count: 1,
        place: CardPlace::Banner,
    });
    for part in [
        "class=\"ds-notification\"",
        "id=\"toast-4\"",
        "Evolution: Lunch?",
        ">5m<",
        ">Lunch?<",
        ">noon<",
        "data-tone=\"strong\"",
        "data-tone=\"italic\"",
        "data-material=\"toast\"",
        ">REPLY<",
    ] {
        assert!(html.contains(part), "{part} in {html}");
    }
    assert!(
        !html.contains(">DEFAULT<"),
        "default is the card's own press"
    );
    for absent in [
        "class=\"ds-notification-count\"",
        "class=\"ds-notification-layer\"",
    ] {
        assert!(!html.contains(absent), "no {absent} alone");
    }
}

#[test]
fn a_group_shows_its_count_and_two_layers() {
    let html = render(Shown {
        note: note(&[]),
        count: 3,
        place: CardPlace::Banner,
    });
    assert!(html.contains(">3<"), "the count");
    assert_eq!(html.matches("class=\"ds-notification-layer\"").count(), 2);
}

#[test]
fn a_center_row_is_on_the_popover_material() {
    let html = render(Shown {
        note: note(&[]),
        count: 1,
        place: CardPlace::Center,
    });
    assert!(html.contains("data-material=\"popover\""));
}
