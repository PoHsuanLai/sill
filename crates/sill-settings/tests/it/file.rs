//! design/22-SETTINGS.md §6 tests 1 and 2 for `sill/settings.toml`: every domain round-trips,
//! and a bad value costs only its own key.

use crate::common;

use common::TempDir;
use ds_settings::{ConfigRoot, Count, Px, Store};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sill_model::screenshots::ShotTool;
use sill_settings::{
    APP, BarSettings, Bounce, ControlCenterSettings, Dnd, DockSettings, LauncherSettings,
    MenusSettings, NotificationsSettings, ScrollSettings, ScrollbarSettings, ShellFile,
    SpacesSettings, SwitcherSettings,
};

/// The store whose `sill` directory is inside `dir`.
fn store(dir: &TempDir) -> Store {
    Store::new(ConfigRoot::Scratch(dir.path().to_owned()), APP)
}

/// Write `settings.toml` (raw bytes) into the store's directory.
fn write(dir: &TempDir, bytes: &[u8]) {
    let app = dir.path().join("sill");
    std::fs::create_dir_all(&app).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(app.join("settings.toml"), bytes).unwrap_or_else(|e| panic!("{e}"));
}

fn load(dir: &TempDir) -> ShellFile {
    store(dir).load::<ShellFile>().value
}

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
        write(&dir, text.as_bytes());
        let got = load(&dir);
        assert_eq!(got, want, "{name}");
        assert_eq!(got.dock.bounce, Bounce::On, "{name}");
    }
}

#[test]
fn missing_or_garbage_files_are_the_defaults() {
    let dir = TempDir::new("garbage");
    assert_eq!(load(&dir), ShellFile::default(), "missing");
    for bytes in [&b""[..], b"not toml [[[", &[0xff, 0xfe]] {
        write(&dir, bytes);
        assert_eq!(load(&dir), ShellFile::default(), "{bytes:?}");
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
        f.screenshot.tool = ShotTool::Command("shoot \"$1\"".to_owned());
    });
    store(&dir).save(&file).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(load(&dir), file);
    let names: Vec<String> = std::fs::read_dir(&target)
        .unwrap_or_else(|e| panic!("{e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        // The writers' lock stays beside the file (quire's update_atomic); no temp file does.
        .filter(|name| name != ".settings.toml.lock")
        .collect();
    assert_eq!(names, ["settings.toml"], "the temp file was renamed away");
}

#[test]
fn unknown_keys_are_reported_and_kept() {
    let dir = TempDir::new("unknown");
    let text = "version = 1\nfuture = \"x\"\n[dock]\npuppy = true\ntile_size_px = 40\n\
                [later]\nanswer = 42\n[control_center]\nmodules = [\"wifi\"]\n";
    write(&dir, text.as_bytes());
    let loaded = store(&dir).load::<ShellFile>();
    let unknown: Vec<&str> = loaded.unknown.iter().map(|k| k.0.as_str()).collect();
    assert_eq!(unknown, ["dock.puppy", "future", "later", "version"]);
    assert!(loaded.invalid.is_empty(), "{:?}", loaded.invalid);
    assert_eq!(loaded.value.dock.tile_size_px, Px(40));

    store(&dir)
        .save(&loaded.value)
        .unwrap_or_else(|e| panic!("{e}"));
    // A save keeps what this version does not know (a newer version's keys survive a round trip).
    let again = store(&dir).load::<ShellFile>();
    let kept: Vec<&str> = again.unknown.iter().map(|k| k.0.as_str()).collect();
    assert_eq!(kept, unknown);
    assert_eq!(again.value, loaded.value);
}

/// A retired key is a key nobody reads: the file that still sets it is told so.
#[test]
fn a_retired_key_is_reported_unknown() {
    let dir = TempDir::new("retired");
    write(
        &dir,
        b"[bar]\ntitle_hit_height_px = 24\n[dock]\nposition = \"bottom\"\nprogress_style = \"ring\"\nbadge_size_px = 18\n[session]\nuser_picture = \"auto\"\n\
          [spaces]\nlookup_order = \"by_id_then_index\"\n[control_center]\nmaterial = \"sheet\"\n\
          [notifications]\nbanner_entry_direction = \"from_right\"\n",
    );
    let loaded = store(&dir).load::<ShellFile>();
    let unknown: Vec<&str> = loaded.unknown.iter().map(|k| k.0.as_str()).collect();
    assert_eq!(
        unknown,
        [
            "bar.title_hit_height_px",
            "control_center.material",
            "dock.badge_size_px",
            "dock.position",
            "dock.progress_style",
            "notifications.banner_entry_direction",
            "session.user_picture",
            "spaces.lookup_order"
        ]
    );
}

/// `screenshot.tool` is parsed at load: `auto` and `off` are words, anything else is the command.
#[test]
fn the_screenshot_tool_is_typed_at_load_and_stored_as_its_text() {
    let dir = TempDir::new("tool");
    let cases: &[(&str, ShotTool)] = &[
        ("auto", ShotTool::Portal),
        (" OFF ", ShotTool::Off),
        ("", ShotTool::Off),
        (
            "grim \"$2/a.png\"",
            ShotTool::Command("grim \"$2/a.png\"".to_owned()),
        ),
    ];
    for (text, want) in cases {
        write(&dir, format!("[screenshot]\ntool = {text:?}\n").as_bytes());
        assert_eq!(load(&dir).screenshot.tool, *want, "{text:?}");
    }
}

/// The idle times are `Secs` and `Mins`: plain integers in the file both ways; a negative one is
/// only that key's default.
#[test]
fn idle_seconds_and_minutes_are_plain_integers_both_ways() {
    use sill_settings::{Mins, Secs};
    let dir = TempDir::new("idle-units");
    let text = "[session]\nlock_grace_s = 12\n\
                [idle]\nscreen_off_ac_min = 20\nscreen_off_battery_min = 3\n\
                suspend_ac_min = 0\nsuspend_battery_min = 45\ndim_s = 9\n\
                locked_screen_off_s = 120\n";
    write(&dir, text.as_bytes());
    let read = load(&dir);
    let want = with(|f| {
        f.session.lock_grace_s = Secs(12);
        f.idle.screen_off_ac_min = Mins(20);
        f.idle.screen_off_battery_min = Mins(3);
        f.idle.suspend_ac_min = Mins(0);
        f.idle.suspend_battery_min = Mins(45);
        f.idle.dim_s = Secs(9);
        f.idle.locked_screen_off_s = Secs(120);
    });
    assert_eq!(read, want);

    store(&dir).save(&read).unwrap_or_else(|e| panic!("{e}"));
    let written = std::fs::read_to_string(dir.path().join("sill").join("settings.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
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
    assert_eq!(load(&dir), want, "{written}");

    write(&dir, b"[idle]\ndim_s = -4\nsuspend_ac_min = 50\n");
    let lenient = load(&dir);
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
