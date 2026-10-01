//! `keyboard.overrides` (design/22-SETTINGS.md §3.25): the typed values a load makes, the rows a
//! load keeps when one is malformed, and the file round trip.

mod common;

use common::TempDir;
use ds_settings::{ConfigRoot, Store};
use sill_settings::{APP, KeyboardOverride, KeyboardSettings, ShellFile, overrides};

fn row(app: &str, profile: &str) -> KeyboardOverride {
    KeyboardOverride {
        app: app.to_owned(),
        profile: profile.to_owned(),
    }
}

fn settings(rows: &[(&str, &str)]) -> KeyboardSettings {
    KeyboardSettings {
        overrides: rows
            .iter()
            .map(|&(app, profile)| row(app, profile))
            .collect(),
        ..KeyboardSettings::default()
    }
}

/// `overrides`' answer as text: the pairs it holds, and what each diagnostic's reason contains.
fn answer(rows: &[(&str, &str)]) -> (Vec<(String, String)>, Vec<String>) {
    let (typed, lines) = overrides(&settings(rows));
    (
        typed
            .0
            .iter()
            .map(|(app, profile)| (app.0.clone(), profile.as_str().to_owned()))
            .collect(),
        lines.into_iter().map(|line| line.reason).collect(),
    )
}

#[test]
fn rows_become_overrides_and_only_a_bad_row_is_dropped() {
    type Case<'a> = (
        &'a str,
        &'a [(&'a str, &'a str)],
        &'a [(&'a str, &'a str)],
        &'a [&'a str],
    );
    const CASES: &[Case] = &[
        ("no rows", &[], &[], &[]),
        (
            "valid rows",
            &[("org.gnome.Nautilus", "files"), ("foot", "term_1")],
            &[("foot", "term_1"), ("org.gnome.Nautilus", "files")],
            &[],
        ),
        (
            "an invalid profile drops its row",
            &[("a", "Has Space"), ("b", "ok")],
            &[("b", "ok")],
            &["`a`'s profile `Has Space` is not valid"],
        ),
        (
            "an empty profile drops its row",
            &[("a", "")],
            &[],
            &["`a`'s profile `` is not valid"],
        ),
        (
            "an empty app drops its row",
            &[("", "files"), ("b", "ok")],
            &[("b", "ok")],
            &["`app` is empty (its profile is `files`)"],
        ),
        (
            "the last row for an app wins",
            &[("a", "one"), ("b", "x"), ("a", "two")],
            &[("a", "two"), ("b", "x")],
            &["`a` has more than one row, the last one is used"],
        ),
        (
            "a bad later row does not replace an earlier good one",
            &[("a", "one"), ("a", "BAD")],
            &[("a", "one")],
            &["`a`'s profile `BAD` is not valid"],
        ),
    ];
    for (case, rows, want, reasons) in CASES {
        let (got, lines) = answer(rows);
        let want: Vec<(String, String)> = want
            .iter()
            .map(|&(a, p)| (a.to_owned(), p.to_owned()))
            .collect();
        assert_eq!(got, want, "{case}");
        assert_eq!(lines.len(), reasons.len(), "{case}: {lines:?}");
        for (line, reason) in lines.iter().zip(*reasons) {
            assert!(line.contains(reason), "{case}: {line:?} lacks {reason:?}");
        }
    }
}

fn store(dir: &TempDir) -> Store {
    Store::new(ConfigRoot::Scratch(dir.path().to_owned()), APP)
}

fn write(dir: &TempDir, text: &str) {
    let app = dir.path().join("sill");
    std::fs::create_dir_all(&app).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(app.join("settings.toml"), text).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn a_malformed_row_costs_only_that_row() {
    let dir = TempDir::new("keyboard-rows");
    write(
        &dir,
        "[[keyboard.overrides]]\napp = \"a\"\nprofile = \"one\"\n\n\
         [[keyboard.overrides]]\napp = 5\nprofile = \"two\"\n\n\
         [[keyboard.overrides]]\napp = \"c\"\n\n\
         [[keyboard.overrides]]\napp = \"d\"\nprofile = \"four\"\n",
    );
    let loaded = store(&dir).load::<ShellFile>();
    assert!(loaded.invalid.is_empty(), "{:?}", loaded.invalid);
    assert!(loaded.unknown.is_empty(), "{:?}", loaded.unknown);
    let keyboard = &loaded.value.keyboard;
    assert_eq!(keyboard.overrides, [row("a", "one"), row("d", "four")]);
    let rejected: Vec<(usize, &str)> = keyboard
        .rejected
        .iter()
        .map(|r| (r.position, r.reason.as_str()))
        .collect();
    assert_eq!(
        rejected,
        [(2, "`app` must be text"), (3, "`profile` is missing")]
    );
    let (typed, lines) = overrides(keyboard);
    assert_eq!(typed.0.len(), 2);
    assert_eq!(lines.len(), 2, "each rejected row is reported: {lines:?}");
    assert!(lines[0].reason.contains("row 2"), "{lines:?}");
    assert!(lines[1].reason.contains("row 3"), "{lines:?}");
}

#[test]
fn a_row_that_is_not_a_table_is_rejected_by_its_place() {
    let dir = TempDir::new("keyboard-not-a-table");
    write(
        &dir,
        "[keyboard]\noverrides = [\"x\", { app = \"a\", profile = \"p\" }]\n",
    );
    let keyboard = store(&dir).load::<ShellFile>().value.keyboard;
    assert_eq!(keyboard.overrides, [row("a", "p")]);
    assert_eq!(keyboard.rejected.len(), 1);
    assert_eq!(keyboard.rejected[0].position, 1);
}

#[test]
fn a_key_that_is_not_a_list_costs_the_key_and_the_default_is_used() {
    let dir = TempDir::new("keyboard-not-a-list");
    write(
        &dir,
        "[keyboard]\noverrides = \"files\"\n\n[dock]\ntile_size_px = 60\n",
    );
    let loaded = store(&dir).load::<ShellFile>();
    assert_eq!(loaded.invalid.len(), 1, "{:?}", loaded.invalid);
    assert_eq!(loaded.invalid[0].path, "keyboard.overrides");
    assert_eq!(loaded.value.keyboard, KeyboardSettings::default());
    assert_eq!(loaded.value.dock.tile_size_px.0, 60, "the other keys stay");
}

#[test]
fn overrides_round_trip_through_the_settings_file() {
    let dir = TempDir::new("keyboard-round-trip");
    let file = ShellFile {
        keyboard: settings(&[("org.gnome.Nautilus", "files"), ("foot", "term")]),
        ..ShellFile::default()
    };
    store(&dir).save(&file).unwrap_or_else(|e| panic!("{e}"));
    let text = std::fs::read_to_string(dir.path().join("sill/settings.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(text.matches("[[keyboard.overrides]]").count(), 2, "{text}");
    assert!(!text.contains("rejected"), "{text}");
    let loaded = store(&dir).load::<ShellFile>();
    assert!(
        loaded.invalid.is_empty() && loaded.unknown.is_empty(),
        "{loaded:?}"
    );
    assert_eq!(loaded.value, file);
}

#[test]
fn the_default_file_has_no_overrides_and_writes_none() {
    let text = toml::to_string(&ShellFile::default().keyboard).unwrap_or_else(|e| panic!("{e}"));
    assert!(!text.contains("rejected"), "{text}");
    assert!(ShellFile::default().keyboard.overrides.is_empty());
}

#[test]
fn the_schema_is_an_advanced_rows_key_with_a_profile_pattern() {
    use ds_settings::schema::{ColumnKind, Exposure, KeyKind, Page};
    let key = sill_settings::schema()
        .key
        .into_iter()
        .find(|key| key.path.0 == "keyboard.overrides")
        .unwrap_or_else(|| panic!("no keyboard.overrides in the schema"));
    assert_eq!(key.exposure, Exposure::Advanced);
    assert_eq!(key.page, Page::KeyboardAndShortcuts);
    let KeyKind::Rows { columns } = key.kind else {
        panic!("not a rows key: {:?}", key.kind);
    };
    let names: Vec<&str> = columns.iter().map(|c| c.name.0.as_str()).collect();
    assert_eq!(names, ["app", "profile"]);
    assert_eq!(
        columns[1].kind,
        ColumnKind::Text {
            pattern: Some("[a-z0-9_-]{1,64}".to_owned())
        }
    );
}
