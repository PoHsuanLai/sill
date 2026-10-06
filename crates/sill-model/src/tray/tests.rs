use super::{
    Check, Tray, TrayEvent, TrayIcon, TrayItem, TrayItemId, TrayMenuEntry, TrayMenuIcon,
    TrayMenuKind, TrayStatus, step,
};
use crate::units::{Availability, Visibility};

fn id(name: &str) -> TrayItemId {
    TrayItemId(format!(":1.{name}/StatusNotifierItem"))
}

fn item(name: &str) -> TrayItem {
    TrayItem {
        id: id(name),
        title: name.to_owned(),
        icon: TrayIcon::Named(format!("{name}-symbolic")),
        status: TrayStatus::Active,
        menu: Vec::new(),
    }
}

fn tray(names: &[&str]) -> Tray {
    Tray {
        items: names.iter().map(|n| item(n)).collect(),
    }
}

fn quit_menu() -> Vec<TrayMenuEntry> {
    vec![TrayMenuEntry {
        id: 7,
        kind: TrayMenuKind::Item {
            label: "Quit".to_owned(),
            enabled: Availability::Enabled,
            visibility: Visibility::Shown,
            check: Check::None,
            icon: TrayMenuIcon::None,
            children: Vec::new(),
            shortcut: None,
            command: None,
        },
    }]
}

#[test]
fn events_reduce_to_the_expected_tray() {
    let with = |name: &str, edit: fn(&mut TrayItem)| {
        let mut t = tray(&["a", "b"]);
        if let Some(i) = t.items.iter_mut().find(|i| i.title == name) {
            edit(i);
        }
        t
    };
    let cases: Vec<(&str, Tray, TrayEvent, Tray)> = vec![
        (
            "registration appends",
            tray(&["a"]),
            TrayEvent::Registered(item("b")),
            tray(&["a", "b"]),
        ),
        (
            "re-registration replaces in place",
            tray(&["a", "b"]),
            TrayEvent::Registered(TrayItem {
                status: TrayStatus::NeedsAttention,
                ..item("a")
            }),
            with("a", |i| i.status = TrayStatus::NeedsAttention),
        ),
        (
            "title",
            tray(&["a", "b"]),
            TrayEvent::Title {
                item: id("b"),
                title: "B!".to_owned(),
            },
            {
                let mut t = tray(&["a", "b"]);
                t.items[1].title = "B!".to_owned();
                t
            },
        ),
        (
            "icon",
            tray(&["a", "b"]),
            TrayEvent::Icon {
                item: id("a"),
                icon: TrayIcon::None,
            },
            with("a", |i| i.icon = TrayIcon::None),
        ),
        (
            "status",
            tray(&["a", "b"]),
            TrayEvent::Status {
                item: id("b"),
                status: TrayStatus::Passive,
            },
            with("b", |i| i.status = TrayStatus::Passive),
        ),
        (
            "menu",
            tray(&["a", "b"]),
            TrayEvent::Menu {
                item: id("a"),
                menu: quit_menu(),
            },
            with("a", |i| i.menu = quit_menu()),
        ),
        (
            "removal",
            tray(&["a", "b"]),
            TrayEvent::Removed(id("a")),
            tray(&["b"]),
        ),
        (
            "a change for an unknown item is dropped",
            tray(&["a"]),
            TrayEvent::Title {
                item: id("zz"),
                title: "x".to_owned(),
            },
            tray(&["a"]),
        ),
        (
            "removing an unknown item changes nothing",
            tray(&["a"]),
            TrayEvent::Removed(id("zz")),
            tray(&["a"]),
        ),
    ];
    for (name, before, event, want) in cases {
        assert_eq!(step(before, event), want, "{name}");
    }
}

#[test]
fn a_scroll_has_a_pinned_wire_line() {
    use super::{ScrollAxis, ScrollDelta, TrayCommand};
    let command = TrayCommand::Scroll {
        item: id("vol"),
        delta: ScrollDelta {
            axis: ScrollAxis::Vertical,
            steps: -2,
        },
    };
    let line = r#"{"kind":"scroll","v":{"item":":1.vol/StatusNotifierItem","delta":{"axis":"vertical","steps":-2}}}"#;
    assert_eq!(serde_json::to_string(&command).ok().as_deref(), Some(line));
    let back: TrayCommand = serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, command);
}

#[test]
fn a_context_menu_ask_has_a_pinned_wire_line() {
    use super::TrayCommand;
    let command = TrayCommand::ContextMenu {
        item: id("fcitx"),
        x: 1900,
        y: 12,
    };
    let line =
        r#"{"kind":"context_menu","v":{"item":":1.fcitx/StatusNotifierItem","x":1900,"y":12}}"#;
    assert_eq!(serde_json::to_string(&command).ok().as_deref(), Some(line));
    let back: TrayCommand = serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, command);
}

#[test]
fn a_menu_with_a_submenu_and_icons_round_trips() {
    let menu = vec![TrayMenuEntry {
        id: 1,
        kind: TrayMenuKind::Item {
            label: "Devices".to_owned(),
            enabled: Availability::Enabled,
            visibility: Visibility::Shown,
            check: Check::Mixed,
            icon: TrayMenuIcon::Named("phone".to_owned()),
            children: vec![TrayMenuEntry {
                id: 2,
                kind: TrayMenuKind::Item {
                    label: "Ping".to_owned(),
                    enabled: Availability::Disabled,
                    visibility: Visibility::Hidden,
                    check: Check::Checked,
                    icon: TrayMenuIcon::Png(vec![137, 80]),
                    children: Vec::new(),
                    shortcut: None,
                    command: None,
                },
            }],
            shortcut: None,
            command: None,
        },
    }];
    let json = serde_json::to_string(&menu).unwrap_or_else(|e| panic!("{e}"));
    let back: Vec<TrayMenuEntry> = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, menu);
}
