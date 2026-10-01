use std::collections::BTreeMap;
use std::path::PathBuf;

use keycap_protocol::ProfileName;
use sill_launcher::{AppId, WindowKey};

use super::{
    CategoryTable, DeclaredProfile, FocusedWindow, ProfileSource, UserOverrides, choose, focused,
    wanted,
};
use crate::apps::{App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal, step};
use crate::cosmic_wl::{Activity, Sizing, Toplevel, Toplevels, WindowState};

fn name(s: &str) -> ProfileName {
    ProfileName::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
}

fn app(id: &str, categories: &[&str], declared: Option<&str>) -> App {
    App {
        id: AppId(id.to_owned()),
        name: id.to_owned(),
        generic_name: String::new(),
        comment: String::new(),
        exec: id.to_owned(),
        icon: AppIcon::None,
        startup_wm_class: None,
        flatpak: None,
        terminal: Terminal::No,
        activation: DbusActivation::Exec,
        actions: Vec::new(),
        mime_types: Vec::new(),
        categories: categories.iter().map(|c| (*c).to_owned()).collect(),
        keyboard_profile: declared.map(name),
        keywords: Vec::new(),
        listing: Listing::Shown,
        path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
    }
}

fn toplevel(key: u64, app_id: &str, activity: Activity) -> Toplevel {
    Toplevel {
        key: WindowKey(key),
        app_id: app_id.to_owned(),
        title: String::new(),
        activity,
        window: WindowState::Normal,
        sizing: Sizing::Floating,
        workspaces: Vec::new(),
        outputs: Vec::new(),
    }
}

fn index() -> AppIndex {
    step(
        AppIndex::default(),
        AppsEvent::Scanned(vec![
            app("foot", &["System", "TerminalEmulator"], None),
            app("game", &["Game"], Some("gaming")),
            app("kitty", &["TerminalEmulator"], Some("kitty-keys")),
            app("plain", &["Utility"], None),
        ]),
    )
}

fn overrides() -> UserOverrides {
    UserOverrides(BTreeMap::from([
        (AppId("foot".to_owned()), name("mine")),
        (AppId("plain".to_owned()), name("plain-mine")),
    ]))
}

/// What one source says for the window of the app with `id`, or for an app the index lacks.
fn alone(source: &dyn ProfileSource, id: &str) -> Option<ProfileName> {
    let index = index();
    let window = toplevel(1, id, Activity::Active);
    source.profile_for(&FocusedWindow {
        toplevel: &window,
        app: index.get(&AppId(id.to_owned())),
    })
}

#[test]
fn each_source_answers_alone() {
    let standard = CategoryTable::standard();
    let overrides = overrides();
    let cases: &[(&str, &dyn ProfileSource, &str, Option<&str>)] = &[
        ("override names the app", &overrides, "foot", Some("mine")),
        ("override has no row", &overrides, "game", None),
        ("override for an unknown app", &overrides, "ghost", None),
        (
            "declared by the entry",
            &DeclaredProfile,
            "game",
            Some("gaming"),
        ),
        ("nothing declared", &DeclaredProfile, "foot", None),
        (
            "declared for an unknown app",
            &DeclaredProfile,
            "ghost",
            None,
        ),
        (
            "a terminal by category",
            &standard,
            "foot",
            Some("terminal"),
        ),
        ("a category outside the table", &standard, "game", None),
        ("a category for an unknown app", &standard, "ghost", None),
    ];
    for &(case, source, id, want) in cases {
        assert_eq!(alone(source, id), want.map(name), "{case}");
    }
}

#[test]
fn the_first_matching_category_row_wins() {
    let table = CategoryTable(vec![
        ("Game".to_owned(), name("gaming")),
        ("TerminalEmulator".to_owned(), name("terminal")),
        ("System".to_owned(), name("system")),
    ]);
    assert_eq!(alone(&table, "foot"), Some(name("terminal")));
}

#[test]
fn a_category_is_a_whole_name() {
    let table = CategoryTable(vec![("Terminal".to_owned(), name("terminal"))]);
    assert_eq!(alone(&table, "foot"), None);
}

#[test]
fn override_beats_declared_beats_category() {
    let overrides = overrides();
    let standard = CategoryTable::standard();
    let all: [&dyn ProfileSource; 3] = [&overrides, &DeclaredProfile, &standard];
    let index = index();
    let cases = [
        ("override beats a terminal category", "foot", "mine"),
        ("declared beats the category", "kitty", "kitty-keys"),
        (
            "override beats a declaration-less entry",
            "plain",
            "plain-mine",
        ),
        ("a declaration alone", "game", "gaming"),
        ("no source: the default", "ghost", "default"),
    ];
    for (case, id, want) in cases {
        let window = toplevel(1, id, Activity::Active);
        let focus = FocusedWindow {
            toplevel: &window,
            app: index.get(&AppId(id.to_owned())),
        };
        assert_eq!(choose(&all, Some(focus)), name(want), "{case}");
    }
}

#[test]
fn source_order_is_the_precedence() {
    let overrides = overrides();
    let standard = CategoryTable::standard();
    let reversed: [&dyn ProfileSource; 3] = [&standard, &DeclaredProfile, &overrides];
    let index = index();
    let window = toplevel(1, "foot", Activity::Active);
    let focus = FocusedWindow {
        toplevel: &window,
        app: index.get(&AppId("foot".to_owned())),
    };
    assert_eq!(choose(&reversed, Some(focus)), name("terminal"));
}

#[test]
fn no_focused_window_is_the_default_profile() {
    let overrides = overrides();
    let sources: [&dyn ProfileSource; 1] = [&overrides];
    assert_eq!(choose(&sources, None), ProfileName::default_profile());
    assert_eq!(choose(&[], None), ProfileName::default_profile());
}

#[test]
fn focused_is_the_active_toplevel_and_its_entry() {
    let index = index();
    let toplevels = |windows: Vec<Toplevel>| Toplevels { windows };
    type Case<'a> = (&'a str, Toplevels, Option<(&'a str, Option<&'a str>)>);
    let cases: Vec<Case> = vec![
        ("no windows", toplevels(vec![]), None),
        (
            "none active",
            toplevels(vec![toplevel(1, "foot", Activity::Inactive)]),
            None,
        ),
        (
            "the active one among others",
            toplevels(vec![
                toplevel(1, "game", Activity::Inactive),
                toplevel(2, "foot", Activity::Active),
            ]),
            Some(("foot", Some("foot"))),
        ),
        (
            "an app the index lacks",
            toplevels(vec![toplevel(3, "org.example.Ghost", Activity::Active)]),
            Some(("org.example.Ghost", None)),
        ),
    ];
    for (case, toplevels, want) in cases {
        let got = focused(&toplevels, &index)
            .map(|w| (w.toplevel.app_id.as_str(), w.app.map(|a| a.id.0.as_str())));
        assert_eq!(got, want, "{case}");
    }
}

#[test]
fn a_terminal_window_chooses_terminal_end_to_end() {
    let index = index();
    let toplevels = Toplevels {
        windows: vec![toplevel(1, "foot", Activity::Active)],
    };
    let standard = CategoryTable::standard();
    let sources: [&dyn ProfileSource; 2] = [&DeclaredProfile, &standard];
    assert_eq!(
        choose(&sources, focused(&toplevels, &index)),
        name("terminal")
    );
}

#[test]
fn the_wanted_profile_follows_the_active_window() {
    let index = index();
    let standard = CategoryTable::standard();
    let none = UserOverrides::default();
    let on = |windows: Vec<Toplevel>| Toplevels { windows };
    let cases: Vec<(&str, &UserOverrides, Toplevels, &str)> = vec![
        ("no window", &none, on(vec![]), "default"),
        (
            "a terminal",
            &none,
            on(vec![toplevel(1, "foot", Activity::Active)]),
            "terminal",
        ),
        (
            "an app declaring its own",
            &none,
            on(vec![toplevel(2, "game", Activity::Active)]),
            "gaming",
        ),
        (
            "an app with nothing to say",
            &none,
            on(vec![toplevel(3, "plain", Activity::Active)]),
            "default",
        ),
        (
            "only the active window counts",
            &none,
            on(vec![
                toplevel(1, "foot", Activity::Inactive),
                toplevel(3, "plain", Activity::Active),
            ]),
            "default",
        ),
    ];
    for (case, overrides, toplevels, want) in cases {
        assert_eq!(
            wanted(overrides, &standard, &toplevels, &index),
            name(want),
            "{case}"
        );
    }
    let mine = overrides();
    let terminal = on(vec![toplevel(1, "foot", Activity::Active)]);
    assert_eq!(
        wanted(&mine, &standard, &terminal, &index),
        name("mine"),
        "the user's override first"
    );
}
