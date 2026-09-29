//! design/22-SETTINGS.md §6 tests 1 and 2 for `sill/settings.toml`: every domain round-trips,
//! and a bad value costs only its own key.

mod common;

use common::TempDir;
use ds_settings::{Count, Px};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sill_settings::{
    BarSettings, Bounce, ControlCenterSettings, Dnd, DockSettings, FILE_NAME, LauncherSettings,
    LoadOutcome, MenusSettings, NotificationsSettings, ScrollSettings, ScrollbarSettings,
    ShellFile, SpacesSettings, SwitcherSettings, load, save, try_load,
};

fn round_trips<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug + Default>() {
    let default = T::default();
    let text = toml::to_string(&default).unwrap_or_else(|e| panic!("{e}"));
    let back: T = toml::from_str(&text).unwrap_or_else(|e| panic!("{text}: {e}"));
    assert_eq!(back, default, "{text}");
}

#[test]
fn every_domain_round_trips() {
    round_trips::<BarSettings>();
    round_trips::<DockSettings>();
    round_trips::<LauncherSettings>();
    round_trips::<ScrollSettings>();
    round_trips::<ScrollbarSettings>();
    round_trips::<MenusSettings>();
    round_trips::<SwitcherSettings>();
    round_trips::<NotificationsSettings>();
    round_trips::<ControlCenterSettings>();
    round_trips::<SpacesSettings>();
    round_trips::<ShellFile>();
}

fn with(edit: impl FnOnce(&mut ShellFile)) -> ShellFile {
    let mut file = ShellFile::default();
    edit(&mut file);
    file
}

#[test]
fn a_bad_value_costs_only_its_own_key() {
    // Every row also sets a sibling away from its default, so a loader that dropped the whole
    // table on the bad value fails it.
    let cases: Vec<(&str, &str, ShellFile)> = vec![
        (
            "an unknown enum word keeps its sibling",
            "[dock]\nbounce = \"bounceyy\"\ntile_size_px = 64\n",
            with(|f| f.dock.tile_size_px = Px(64)),
        ),
        (
            "a negative px is the default and keeps its sibling",
            "[bar]\nheight_px = -3\nstatus_gap_px = 6\n",
            with(|f| f.bar.status_gap_px = Px(6)),
        ),
        (
            "a missing table is that table's defaults",
            "[notifications]\ndnd = \"on\"\n",
            with(|f| f.notifications.dnd = Dnd::On),
        ),
        (
            "a table that is not a table is its defaults",
            "dock = 7\n[spaces]\ndefault_grain = 55\n",
            with(|f| f.spaces.default_grain = Count(55)),
        ),
        (
            "a list of unknown modules is the default list",
            "[control_center]\nmodules = [\"toaster\"]\nwidth_px = 300\n",
            with(|f| f.control_center.width_px = Px(300)),
        ),
    ];
    let dir = TempDir::new("lenient");
    for (name, text, want) in cases {
        std::fs::write(dir.path().join(FILE_NAME), text).unwrap_or_else(|e| panic!("{e}"));
        let got = load(dir.path());
        assert_eq!(got, want, "{name}");
        assert_eq!(got.dock.bounce, Bounce::On, "{name}");
    }
}

#[test]
fn missing_or_garbage_files_are_the_defaults() {
    let dir = TempDir::new("garbage");
    assert_eq!(load(dir.path()), ShellFile::default(), "missing");
    for bytes in [&b""[..], b"not toml [[[", &[0xff, 0xfe]] {
        std::fs::write(dir.path().join(FILE_NAME), bytes).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(dir.path()), ShellFile::default(), "{bytes:?}");
    }
}

#[test]
fn a_save_is_one_file_and_reads_back() {
    let dir = TempDir::new("save");
    let target = dir.path().join("sill");
    let file = with(|f| {
        f.dock.autohide = sill_settings::AutoHide::On;
        f.bar.height_px = Px(28);
        f.launcher.web_engine_url = "https://example.org/?q={query}".to_owned();
    });
    save(&target, &file).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(load(&target), file);
    let names: Vec<String> = std::fs::read_dir(&target)
        .unwrap_or_else(|e| panic!("{e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, [FILE_NAME], "the temp file was renamed away");
}

#[test]
fn unknown_keys_and_tables_survive_a_round_trip() {
    let dir = TempDir::new("unknown");
    let text = "version = 1\nfuture = \"kept\"\n[dock]\npuppy = true\ntile_size_px = 40\n\
                [later]\nanswer = 42\n";
    std::fs::write(dir.path().join(FILE_NAME), text).unwrap_or_else(|e| panic!("{e}"));
    let read = load(dir.path());
    assert_eq!(read.dock.tile_size_px, Px(40));
    save(dir.path(), &read).unwrap_or_else(|e| panic!("{e}"));
    let written =
        std::fs::read_to_string(dir.path().join(FILE_NAME)).unwrap_or_else(|e| panic!("{e}"));
    let tree: toml::Table = toml::from_str(&written).unwrap_or_else(|e| panic!("{e}"));
    let at = |table: &str, key: &str| tree.get(table).and_then(|t| t.get(key)).cloned();
    assert_eq!(
        at("dock", "puppy"),
        Some(toml::Value::Boolean(true)),
        "{written}"
    );
    assert_eq!(
        at("later", "answer"),
        Some(toml::Value::Integer(42)),
        "{written}"
    );
    assert_eq!(
        tree.get("future"),
        Some(&toml::Value::String("kept".to_owned())),
        "{written}"
    );
}

/// Q445: the idle times became `Secs` and `Mins` after files were written with them as `Count`.
/// Both are plain integers in the file, so an existing file loads unchanged, and a save writes
/// them back as the same plain integers; a negative one is only that key's default.
#[test]
fn idle_seconds_and_minutes_are_plain_integers_both_ways() {
    use sill_settings::{IdleTimesFrom, Mins, Secs};
    let dir = TempDir::new("idle-units");
    let text = "[session]\nlock_grace_s = 12\n\
                [idle]\ntimes = \"sill\"\nscreen_off_ac_min = 20\nscreen_off_battery_min = 3\n\
                suspend_ac_min = 0\nsuspend_battery_min = 45\ndim_s = 9\n\
                locked_screen_off_s = 120\n";
    std::fs::write(dir.path().join(FILE_NAME), text).unwrap_or_else(|e| panic!("{e}"));
    let read = load(dir.path());
    let want = with(|f| {
        f.session.lock_grace_s = Secs(12);
        f.idle.times = IdleTimesFrom::Sill;
        f.idle.screen_off_ac_min = Mins(20);
        f.idle.screen_off_battery_min = Mins(3);
        f.idle.suspend_ac_min = Mins(0);
        f.idle.suspend_battery_min = Mins(45);
        f.idle.dim_s = Secs(9);
        f.idle.locked_screen_off_s = Secs(120);
    });
    assert_eq!(read, want);

    save(dir.path(), &read).unwrap_or_else(|e| panic!("{e}"));
    let written =
        std::fs::read_to_string(dir.path().join(FILE_NAME)).unwrap_or_else(|e| panic!("{e}"));
    let tree: toml::Table = toml::from_str(&written).unwrap_or_else(|e| panic!("{e}"));
    let at = |table: &str, key: &str| tree.get(table).and_then(|t| t.get(key)).cloned();
    const WRITTEN: &[(&str, &str, i64)] = &[
        ("session", "lock_grace_s", 12),
        ("idle", "screen_off_ac_min", 20),
        ("idle", "screen_off_battery_min", 3),
        ("idle", "suspend_ac_min", 0),
        ("idle", "suspend_battery_min", 45),
        ("idle", "dim_s", 9),
        ("idle", "locked_screen_off_s", 120),
    ];
    for &(table, key, value) in WRITTEN {
        assert_eq!(
            at(table, key),
            Some(toml::Value::Integer(value)),
            "{table}.{key}: {written}"
        );
    }
    assert_eq!(load(dir.path()), want, "{written}");

    std::fs::write(
        dir.path().join(FILE_NAME),
        "[idle]\ndim_s = -4\nsuspend_ac_min = 50\n",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let lenient = load(dir.path());
    assert_eq!(
        lenient.idle.dim_s,
        Secs(5),
        "a negative seconds is the default"
    );
    assert_eq!(
        lenient.idle.suspend_ac_min,
        Mins(50),
        "and keeps its sibling"
    );
}

/// `widgets.sizes` is gone (each widget has one size, FINDINGS F1024): a file an earlier build
/// wrote with it loads with every other widget key read, and the old line is kept unread.
#[test]
fn a_stored_widget_sizes_line_is_ignored() {
    use sill_settings::{DesktopWidgets, WidgetKind};
    let dir = TempDir::new("widget-sizes");
    let text = "[widgets]\ndesktop = \"on\"\ncenter = [\"battery\"]\n\
                sizes = \"desktop.calendar=medium; center.up_next=large\"\n";
    std::fs::write(dir.path().join(FILE_NAME), text).unwrap_or_else(|e| panic!("{e}"));
    let read = load(dir.path());
    let want = with(|f| {
        f.widgets.desktop = DesktopWidgets::On;
        f.widgets.center = vec![WidgetKind::Battery];
        f.widgets.extra.insert(
            "sizes".to_owned(),
            toml::Value::String("desktop.calendar=medium; center.up_next=large".to_owned()),
        );
    });
    assert_eq!(read, want);
}

/// [`try_load`]'s change-to-dispatch mapping for the watch (FINDINGS F993, F1030): a bad value
/// inside a table is still only that key ([`LoadOutcome::Read`], as [`load`] treats it — the
/// watch may still publish it), text that is not TOML at all is [`LoadOutcome::Unparseable`]
/// (the watch must keep the last good file rather than publish this), and a missing file is
/// still every setting's default (nothing was ever good to keep).
#[test]
fn try_load_tells_a_bad_value_from_a_file_that_is_not_toml_at_all() {
    let dir = TempDir::new("try-load");
    assert_eq!(
        try_load(dir.path()),
        LoadOutcome::Read(Box::default()),
        "missing file"
    );

    std::fs::write(
        dir.path().join(FILE_NAME),
        "[notifications]\ndnd = \"on\"\n",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        try_load(dir.path()),
        LoadOutcome::Read(Box::new(with(|f| f.notifications.dnd = Dnd::On))),
        "a valid table with a good value"
    );

    std::fs::write(
        dir.path().join(FILE_NAME),
        "[dock]\nbounce = \"bounceyy\"\ntile_size_px = 64\n",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        try_load(dir.path()),
        LoadOutcome::Read(Box::new(with(|f| f.dock.tile_size_px = Px(64)))),
        "a bad value costs only its own key, same as `load`"
    );

    for text in ["not toml [[[ = =", "\u{0}\u{1}garbage"] {
        std::fs::write(dir.path().join(FILE_NAME), text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(try_load(dir.path()), LoadOutcome::Unparseable, "{text:?}");
    }
}
