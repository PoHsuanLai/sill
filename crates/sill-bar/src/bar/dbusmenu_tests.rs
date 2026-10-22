//! A dbusmenu layout as menu entries: nesting, rules, marks, flags, icons, and a layout that
//! changes while its menu is open.

use ds::components::menus::item::item::MenuImage;
use ds::prelude::Check as DsCheck;
use ds::prelude::*;
use sill_model::tray::{
    Check, Tray, TrayEvent, TrayIcon, TrayItem, TrayItemId, TrayMenuEntry, TrayMenuIcon,
    TrayMenuKind, TrayStatus, step,
};
use sill_model::units::{Availability as Enabled, Visibility};

use super::{has_rows, menu_items};
use crate::bar::pick::BarPick;

/// What a row of the test item picks.
fn tray_pick(entry: i32) -> BarPick {
    BarPick::Tray {
        item: item_id(),
        entry,
    }
}

fn item_id() -> TrayItemId {
    TrayItemId(":1.9/StatusNotifierItem".to_owned())
}

/// A row with every other field plain.
struct Row {
    id: i32,
    label: &'static str,
    enabled: Enabled,
    visibility: Visibility,
    check: Check,
    icon: TrayMenuIcon,
    children: Vec<TrayMenuEntry>,
}

fn row(id: i32, label: &'static str) -> Row {
    Row {
        id,
        label,
        enabled: Enabled::Enabled,
        visibility: Visibility::Shown,
        check: Check::None,
        icon: TrayMenuIcon::None,
        children: Vec::new(),
    }
}

impl Row {
    fn entry(self) -> TrayMenuEntry {
        TrayMenuEntry {
            id: self.id,
            kind: TrayMenuKind::Item {
                label: self.label.to_owned(),
                enabled: self.enabled,
                visibility: self.visibility,
                check: self.check,
                icon: self.icon,
                children: self.children,
                shortcut: None,
                command: None,
            },
        }
    }
}

fn rule(id: i32) -> TrayMenuEntry {
    TrayMenuEntry {
        id,
        kind: TrayMenuKind::Separator,
    }
}

/// The fake theme: a name resolves to a glyph, bytes to another.
fn fake_icon(icon: &TrayMenuIcon) -> Option<MenuImage> {
    match icon {
        TrayMenuIcon::None => None,
        TrayMenuIcon::Named(_) => Some(MenuImage::Icon(Icon::Window)),
        TrayMenuIcon::Png(_) => Some(MenuImage::Icon(Icon::Switches)),
    }
}

/// One line per entry, submenus indented in brackets: `[x]` on, `[ ]` off, `[-]` mixed, `@` an
/// icon, `~` disabled, `#id` the dbusmenu id a pick sends.
fn describe(entries: &[MenuItem<BarPick>]) -> String {
    entries
        .iter()
        .map(|entry| match entry {
            MenuItem::Separator => "--".to_owned(),
            MenuItem::Header(text) => format!("# {text}"),
            MenuItem::Info { title, .. } => format!("i {title}"),
            MenuItem::Swatches(row) => format!("* {}", row.title),
            MenuItem::Item {
                value,
                title,
                image,
                check,
                availability,
                ..
            } => {
                let mark = match check {
                    None => "",
                    Some(DsCheck::On) => "[x] ",
                    Some(DsCheck::Off) => "[ ] ",
                    Some(DsCheck::Mixed) => "[-] ",
                };
                let id = match value {
                    BarPick::Tray { entry, .. } => *entry,
                    other => panic!("not a tray pick: {other:?}"),
                };
                let icon = if image.is_some() { "@" } else { "" };
                let off = if *availability == Availability::Disabled {
                    "~"
                } else {
                    ""
                };
                format!("{off}{mark}{icon}{title}#{id}")
            }
            MenuItem::Submenu {
                title,
                image,
                availability,
                children,
            } => {
                let icon = if image.is_some() { "@" } else { "" };
                let off = if *availability == Availability::Disabled {
                    "~"
                } else {
                    ""
                };
                format!("{off}{icon}{title} > [{}]", describe(children))
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

#[test]
fn layouts_become_entries() {
    let cases: Vec<(&str, Vec<TrayMenuEntry>, &str)> = vec![
        ("an empty layout", vec![], ""),
        (
            "plain rows keep their dbusmenu id",
            vec![row(3, "Open").entry(), row(8, "Quit").entry()],
            "Open#3 | Quit#8",
        ),
        (
            "a submenu nests, to any depth",
            vec![
                row(1, "Devices")
                    .with_children(vec![
                        row(2, "Pixel")
                            .with_children(vec![row(3, "Ping").entry(), row(4, "Share").entry()])
                            .entry(),
                        row(5, "Refresh").entry(),
                    ])
                    .entry(),
                row(6, "Quit").entry(),
            ],
            "Devices > [Pixel > [Ping#3 | Share#4] | Refresh#5] | Quit#6",
        ),
        (
            "rules stay between rows",
            vec![
                rule(10),
                row(1, "A").entry(),
                rule(11),
                rule(12),
                row(2, "B").entry(),
                rule(13),
            ],
            "A#1 | -- | B#2",
        ),
        (
            "hidden rows go, and the rules they leave doubled collapse",
            vec![
                row(1, "A").entry(),
                rule(10),
                Row {
                    visibility: Visibility::Hidden,
                    ..row(2, "Gone")
                }
                .entry(),
                rule(11),
                row(3, "B").entry(),
            ],
            "A#1 | -- | B#3",
        ),
        (
            "a hidden child is dropped, and a parent left with none is a plain row",
            vec![
                row(1, "Parent")
                    .with_children(vec![
                        Row {
                            visibility: Visibility::Hidden,
                            ..row(2, "Gone")
                        }
                        .entry(),
                    ])
                    .entry(),
                row(3, "Other")
                    .with_children(vec![row(4, "Kept").entry()])
                    .entry(),
            ],
            "Parent#1 | Other > [Kept#4]",
        ),
        (
            "toggles and radios are marks: on, off, mixed",
            vec![
                Row {
                    check: Check::Checked,
                    ..row(1, "Mute")
                }
                .entry(),
                Row {
                    check: Check::Unchecked,
                    ..row(2, "Left")
                }
                .entry(),
                Row {
                    check: Check::Mixed,
                    ..row(3, "Some")
                }
                .entry(),
            ],
            "[x] Mute#1 | [ ] Left#2 | [-] Some#3",
        ),
        (
            "disabled rows are drawn disabled, parents too",
            vec![
                Row {
                    enabled: Enabled::Disabled,
                    ..row(1, "Paste")
                }
                .entry(),
                Row {
                    enabled: Enabled::Disabled,
                    ..row(2, "More")
                }
                .with_children(vec![row(3, "x").entry()])
                .entry(),
            ],
            "~Paste#1 | ~More > [x#3]",
        ),
        (
            "icons come from a name or from bytes, on rows and parents",
            vec![
                Row {
                    icon: TrayMenuIcon::Named("mail".to_owned()),
                    ..row(1, "Mail")
                }
                .entry(),
                Row {
                    icon: TrayMenuIcon::Png(vec![1]),
                    ..row(2, "Pic")
                }
                .with_children(vec![row(3, "x").entry()])
                .entry(),
                row(4, "Bare").entry(),
            ],
            "@Mail#1 | @Pic > [x#3] | Bare#4",
        ),
    ];
    for (name, rows, want) in cases {
        let got = describe(&menu_items(&tray_pick, &rows, &fake_icon));
        assert_eq!(got, want, "{name}");
    }
}

impl Row {
    fn with_children(self, children: Vec<TrayMenuEntry>) -> Row {
        Row { children, ..self }
    }
}

#[test]
fn only_a_shown_row_makes_a_menu() {
    let hidden = Row {
        visibility: Visibility::Hidden,
        ..row(1, "Gone")
    }
    .entry();
    let cases: Vec<(&str, Vec<TrayMenuEntry>, bool)> = vec![
        ("nothing", vec![], false),
        ("rules only", vec![rule(1)], false),
        ("hidden only", vec![hidden.clone(), rule(2)], false),
        ("one shown row", vec![hidden, row(3, "Open").entry()], true),
    ];
    for (name, rows, want) in cases {
        assert_eq!(has_rows(&rows), want, "{name}");
    }
}

fn tray_with(menu: Vec<TrayMenuEntry>) -> Tray {
    Tray {
        items: vec![TrayItem {
            id: item_id(),
            title: "Telegram".to_owned(),
            icon: TrayIcon::None,
            status: TrayStatus::Active,
            menu,
        }],
    }
}

fn shown(tray: &Tray) -> String {
    describe(&menu_items(&tray_pick, &tray.items[0].menu, &fake_icon))
}

#[test]
fn a_layout_updated_while_the_menu_is_open_replaces_the_rows() {
    let before = tray_with(vec![
        row(1, "Show").entry(),
        row(2, "Mute")
            .with_children(vec![row(3, "For an hour").entry()])
            .entry(),
    ]);
    assert_eq!(shown(&before), "Show#1 | Mute > [For an hour#3]");
    // The app sends LayoutUpdated: the submenu grows a row, Show becomes a toggle and a row is
    // added; the ids that stay keep their picks.
    let after = step(
        before.clone(),
        TrayEvent::Menu {
            item: item_id(),
            menu: vec![
                Row {
                    check: Check::Checked,
                    ..row(1, "Show")
                }
                .entry(),
                row(2, "Mute")
                    .with_children(vec![
                        row(3, "For an hour").entry(),
                        row(4, "For a day").entry(),
                    ])
                    .entry(),
                row(5, "Quit").entry(),
            ],
        },
    );
    assert_eq!(
        shown(&after),
        "[x] Show#1 | Mute > [For an hour#3 | For a day#4] | Quit#5"
    );
    assert_ne!(shown(&before), shown(&after));
}
