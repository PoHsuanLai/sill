use std::collections::BTreeMap;

use docket_core::ActionRef;
use prov::{ActionName, AppName, Effect};
use sill_launcher::WindowKey;

use super::{
    ActionIndex, AppMenuEvent, AppMenus, Follower, Followers, Following, Identity, ItemKey,
    MenuApp, MenuName, Node, NodeProps, NodeType, OwnItem, QuitRow, Route, ServedMenu, ToggleKind,
    effect_of, listing, own_commands, own_groups, route_of, rows_of, step, titles,
};
use crate::ids::OutputName;
use crate::tray::{Check, Chord, MenuCommand, TrayMenuEntry, TrayMenuIcon, TrayMenuKind};
use crate::units::{Availability, Visibility};
use crate::windows::{Activity, Sizing, Toplevel, Toplevels, WindowState, WlCommand};

fn node(id: i32, label: &str, children: Vec<Node>) -> Node {
    Node {
        id,
        props: NodeProps {
            label: Some(label.to_owned()),
            ..NodeProps::default()
        },
        children,
    }
}

fn with(mut node: Node, edit: impl FnOnce(&mut NodeProps)) -> Node {
    edit(&mut node.props);
    node
}

/// A quire-shaped bar: the app's own menu, Edit with Undo, a hidden menu, and View.
fn served() -> ServedMenu {
    let undo = with(node(21, "Undo", Vec::new()), |p| {
        p.shortcut = Some(Chord(vec!["Super".to_owned(), "Z".to_owned()]));
        p.command = Some(MenuCommand("standard.undo".to_owned()));
    });
    let sidebar = with(node(41, "Sidebar", Vec::new()), |p| {
        p.toggle = ToggleKind::Checkmark;
        p.toggle_state = 1;
    });
    let menus = vec![
        node(11, "Notes", vec![node(12, "About Notes", Vec::new())]),
        node(20, "Edit", vec![undo]),
        with(
            node(30, "Secret", vec![node(31, "Hidden", Vec::new())]),
            |p| {
                p.visible = Visibility::Hidden;
            },
        ),
        node(32, "Empty", Vec::new()),
        node(40, "View", vec![sidebar]),
    ];
    ServedMenu {
        name: MenuName("org.quire.AppMenu.dev.notes".to_owned()),
        menus: rows_of(menus),
    }
}

#[test]
fn a_layout_becomes_the_trays_rows() {
    let rule = Node {
        id: 5,
        props: NodeProps {
            kind: NodeType::Separator,
            ..NodeProps::default()
        },
        children: Vec::new(),
    };
    let radio = with(node(6, "_Small", Vec::new()), |p| {
        p.toggle = ToggleKind::Radio;
        p.toggle_state = 0;
        p.enabled = Availability::Disabled;
        p.icon_name = Some("zoom-out".to_owned());
    });
    let mixed = with(node(7, "Both", Vec::new()), |p| {
        p.toggle = ToggleKind::Checkmark;
    });
    let rows = rows_of(vec![rule, radio, mixed, node(8, "snake__case", Vec::new())]);
    let kinds: Vec<(i32, String)> = rows
        .iter()
        .map(|row| match &row.kind {
            TrayMenuKind::Separator => (row.id, "rule".to_owned()),
            TrayMenuKind::Item {
                label,
                enabled,
                check,
                icon,
                ..
            } => (
                row.id,
                format!(
                    "{label}|{enabled:?}|{check:?}|{}",
                    icon != &TrayMenuIcon::None
                ),
            ),
        })
        .collect();
    let want = [
        (5, "rule"),
        (6, "Small|Disabled|Unchecked|true"),
        (7, "Both|Enabled|Mixed|false"),
        (8, "snake_case|Enabled|None|false"),
    ];
    let want: Vec<(i32, String)> = want.iter().map(|(i, s)| (*i, (*s).to_owned())).collect();
    assert_eq!(kinds, want);
}

#[test]
fn the_first_menu_is_the_apps_own_and_hidden_or_empty_menus_make_no_title() {
    let made = titles(&served());
    let app = made.app.as_ref().map(|(id, rows)| (*id, rows.len()));
    assert_eq!(
        app,
        Some((11, 1)),
        "the first menu goes into the app-name menu"
    );
    let order: Vec<(&str, i32)> = made
        .menus
        .iter()
        .map(|menu| (menu.title.as_str(), menu.id))
        .collect();
    assert_eq!(order, [("Edit", 20), ("View", 40)]);
    assert_eq!(
        titles(&ServedMenu {
            name: MenuName(String::new()),
            menus: Vec::new(),
        }),
        super::Titles::default()
    );
}

fn app(name: &str) -> MenuApp {
    MenuApp(name.to_owned())
}

fn bar(name: &str) -> Follower {
    Follower(name.to_owned())
}

#[test]
fn followers_start_an_app_once_and_stop_it_when_nobody_names_it() {
    type Step = (Follower, Option<MenuApp>, Vec<MenuApp>, Vec<MenuApp>);
    let cases: Vec<(&str, Step)> = vec![
        (
            "the first bar starts its app",
            (bar("DP-1"), Some(app("a")), vec![app("a")], vec![]),
        ),
        (
            "a second bar on the same app starts nothing",
            (bar("DP-2"), Some(app("a")), vec![], vec![]),
        ),
        (
            "a bar moving to another app starts it, the first stays",
            (bar("DP-2"), Some(app("b")), vec![app("b")], vec![]),
        ),
        (
            "the last bar leaving stops the app",
            (bar("DP-1"), None, vec![], vec![app("a")]),
        ),
        (
            "naming the same app again changes nothing",
            (bar("DP-2"), Some(app("b")), vec![], vec![]),
        ),
    ];
    let mut followers = Followers::default();
    for (name, (by, wants, start, stop)) in cases {
        let (next, following) = followers.set(by, wants);
        assert_eq!(following, Following { start, stop }, "{name}");
        followers = next;
    }
    assert_eq!(
        followers.apps().into_iter().collect::<Vec<_>>(),
        [&app("b")]
    );
}

#[test]
fn the_published_menus_follow_what_was_heard() {
    let served = served();
    let seen = step(
        AppMenus::default(),
        AppMenuEvent::Served {
            app: app("dev.notes"),
            menu: served.clone(),
        },
    );
    assert_eq!(seen.served.get(&app("dev.notes")), Some(&served));
    let gone = step(seen, AppMenuEvent::Gone(app("dev.notes")));
    assert_eq!(gone, AppMenus::default());
}

fn window(key: u64, app_id: &str, state: WindowState) -> Toplevel {
    Toplevel {
        key: WindowKey(key),
        app_id: app_id.to_owned(),
        title: format!("{app_id} {key}"),
        activity: Activity::Inactive,
        window: state,
        sizing: Sizing::Floating,
        workspaces: Vec::new(),
        outputs: vec![OutputName("DP-1".to_owned())],
    }
}

#[test]
fn the_shells_own_rows_become_compositor_commands() {
    let toplevels = Toplevels {
        windows: vec![
            window(1, "foot", WindowState::Normal),
            window(2, "foot", WindowState::Minimized),
            window(3, "files", WindowState::Normal),
            window(4, "files", WindowState::Minimized),
        ],
    };
    let cases: Vec<(OwnItem, Vec<WlCommand>)> = vec![
        (
            OwnItem::Raise(WindowKey(9)),
            vec![WlCommand::Activate(WindowKey(9))],
        ),
        (
            OwnItem::Minimise(WindowKey(2)),
            vec![WlCommand::Minimize(WindowKey(2))],
        ),
        (OwnItem::Hide, vec![WlCommand::Minimize(WindowKey(1))]),
        (OwnItem::HideOthers, vec![WlCommand::Minimize(WindowKey(3))]),
        (
            OwnItem::ShowAll,
            vec![
                WlCommand::Unminimize(WindowKey(2)),
                WlCommand::Unminimize(WindowKey(4)),
            ],
        ),
        (
            OwnItem::Quit,
            vec![
                WlCommand::Close(WindowKey(1)),
                WlCommand::Close(WindowKey(2)),
            ],
        ),
    ];
    for (item, want) in cases {
        assert_eq!(own_commands(item, "foot", &toplevels), want, "{item:?}");
    }
}

#[test]
fn the_own_groups_hold_windows_then_hiding_then_quit() {
    let windows = [(WindowKey(4), "~".to_owned())];
    let labels = |quit| {
        own_groups("Foot", &windows, quit)
            .into_iter()
            .map(|group| group.into_iter().map(|row| row.label).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    };
    let hide = ["Minimise", "Hide Foot", "Hide Others", "Show All"]
        .map(str::to_owned)
        .to_vec();
    assert_eq!(
        labels(QuitRow::Offered),
        [
            vec!["~".to_owned()],
            hide.clone(),
            vec!["Quit Foot".to_owned()]
        ]
    );
    assert_eq!(labels(QuitRow::LeftToApp), [vec!["~".to_owned()], hide]);
}

fn name(text: &str) -> AppName {
    AppName::parse(text).unwrap_or_else(|e| panic!("{e}"))
}

fn action(app: &str, action: &str) -> ActionRef {
    ActionRef {
        app: name(app),
        name: ActionName::parse(action).unwrap_or_else(|e| panic!("{e}")),
    }
}

/// The manifests as the table sees them: the shell's harmless action and the notes app's own.
fn actions() -> ActionIndex {
    ActionIndex(BTreeMap::from([
        (
            action("org.quire.Shell", "shell.workspace.switch"),
            Effect::Read,
        ),
        (action("dev.notes", "note.archive"), Effect::UndoableWrite),
        (action("dev.notes", "note.erase"), Effect::Destructive),
    ]))
}

#[test]
fn an_activation_costs_what_its_route_says() {
    let notes = Identity::Verified(name("dev.notes"));
    let command = |text: &str| Some(MenuCommand(text.to_owned()));
    type Case<'a> = (
        &'a str,
        ItemKey,
        Option<MenuCommand>,
        Identity,
        Route,
        Effect,
    );
    let cases: Vec<Case> = vec![
        (
            "a verified app's own action takes that action's declared effect",
            ItemKey::Menu(7),
            command("note.archive"),
            notes.clone(),
            Route::Intents(action("dev.notes", "note.archive")),
            Effect::UndoableWrite,
        ),
        (
            "and a destructive one stays destructive",
            ItemKey::Menu(7),
            command("note.erase"),
            notes.clone(),
            Route::Intents(action("dev.notes", "note.erase")),
            Effect::Destructive,
        ),
        (
            "another app's harmless action named by an item is only a claim",
            ItemKey::Menu(7),
            command("shell.workspace.switch"),
            notes.clone(),
            Route::Click(7),
            Effect::Destructive,
        ),
        (
            "an id the verified app's manifest does not have is a click",
            ItemKey::Menu(7),
            command("note.unknown"),
            notes.clone(),
            Route::Click(7),
            Effect::Destructive,
        ),
        (
            "an unverified app's id counts for nothing even when its manifest has it",
            ItemKey::Menu(7),
            command("note.archive"),
            Identity::Unverified,
            Route::Click(7),
            Effect::Destructive,
        ),
        (
            "an item with no command is a click",
            ItemKey::Menu(7),
            None,
            notes.clone(),
            Route::Click(7),
            Effect::Destructive,
        ),
        (
            "a command that is no action name is a click",
            ItemKey::Menu(7),
            command("Not A Name"),
            notes,
            Route::Click(7),
            Effect::Destructive,
        ),
        (
            "the shell's own Hide is read",
            ItemKey::Own(OwnItem::Hide),
            None,
            Identity::Unverified,
            Route::Own(OwnItem::Hide),
            Effect::Read,
        ),
        (
            "minimising a window is read",
            ItemKey::Own(OwnItem::Minimise(WindowKey(3))),
            None,
            Identity::Unverified,
            Route::Own(OwnItem::Minimise(WindowKey(3))),
            Effect::Read,
        ),
        (
            "raising a window is read",
            ItemKey::Own(OwnItem::Raise(WindowKey(3))),
            None,
            Identity::Unverified,
            Route::Own(OwnItem::Raise(WindowKey(3))),
            Effect::Read,
        ),
        (
            "Quit cannot be taken back, so it asks",
            ItemKey::Own(OwnItem::Quit),
            None,
            Identity::Unverified,
            Route::Own(OwnItem::Quit),
            Effect::Destructive,
        ),
    ];
    let actions = actions();
    for (why, item, command, identity, route, effect) in cases {
        let got = route_of(item, command.as_ref(), &identity, &actions);
        assert_eq!(got, route, "{why}");
        assert_eq!(effect_of(&got, &actions), effect, "{why}");
    }
}

#[test]
fn an_item_key_reads_back_what_a_listing_wrote() {
    let cases = [
        ItemKey::Own(OwnItem::Raise(WindowKey(12))),
        ItemKey::Own(OwnItem::Minimise(WindowKey(5))),
        ItemKey::Own(OwnItem::Hide),
        ItemKey::Own(OwnItem::HideOthers),
        ItemKey::Own(OwnItem::ShowAll),
        ItemKey::Own(OwnItem::Quit),
        ItemKey::Menu(-1_234_567),
        ItemKey::Menu(0),
        ItemKey::Menu(88),
    ];
    for key in cases {
        assert_eq!(ItemKey::parse(&key.text()), Some(key), "{key:?}");
    }
    for text in ["", "raise:", "raise:x", "zoom:3", "Quit", "12x"] {
        assert_eq!(ItemKey::parse(text), None, "{text:?}");
    }
}

#[test]
fn the_listing_has_one_line_per_row_with_the_id_first() {
    let own = own_groups(
        "Notes",
        &[(WindowKey(4), "Inbox".to_owned())],
        QuitRow::LeftToApp,
    );
    let text = listing("Notes", &own, Some(&served()));
    let want = "\
Notes
  [12] About Notes
  [raise:4] Inbox
  [minimise:4] Minimise
  [hide] Hide Notes
  [hide-others] Hide Others
  [show-all] Show All
Edit
  [21] Undo keys=Super+Z command=standard.undo
View
  [41] Sidebar (checked)
";
    assert_eq!(text, want);
}

#[test]
fn a_long_listing_ends_at_a_whole_line_with_a_mark() {
    let rows: Vec<Node> = (0..2000)
        .map(|i| node(i, "A row with some words", Vec::new()))
        .collect();
    let big = ServedMenu {
        name: MenuName(String::new()),
        menus: rows_of(vec![node(1, "App", Vec::new()), node(2, "File", rows)]),
    };
    let text = listing("Big", &[], Some(&big));
    assert!(
        text.chars().count() <= super::LISTING_MAX,
        "{}",
        text.chars().count()
    );
    assert!(
        text.ends_with("(more rows left out)"),
        "{}",
        &text[text.len() - 60..]
    );
    assert!(
        text.lines()
            .rev()
            .nth(1)
            .is_some_and(|l| l.ends_with("A row with some words"))
    );
}

#[test]
fn the_new_tray_row_fields_round_trip() {
    let row = TrayMenuEntry {
        id: 3,
        kind: TrayMenuKind::Item {
            label: "Undo".to_owned(),
            enabled: Availability::Enabled,
            visibility: Visibility::Shown,
            check: Check::None,
            icon: TrayMenuIcon::None,
            children: Vec::new(),
            shortcut: Some(Chord(vec!["Super".to_owned(), "Z".to_owned()])),
            command: Some(MenuCommand("standard.undo".to_owned())),
        },
    };
    let json = serde_json::to_string(&row).unwrap_or_else(|e| panic!("{e}"));
    assert!(json.contains(r#""shortcut":["Super","Z"]"#), "{json}");
    assert!(json.contains(r#""command":"standard.undo""#), "{json}");
    let back: TrayMenuEntry = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, row);
}
