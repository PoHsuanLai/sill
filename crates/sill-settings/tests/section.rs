//! `Section`: each domain's `KEY` is its table in the file, and `of` finds it.

use sill_settings::{
    BarSettings, CalendarSettings, ControlCenterSettings, DisplaySettings, DockSettings,
    HotCornersSettings, IdleSettings, KeyboardOverride, KeyboardSettings, LauncherSettings,
    MenusSettings, NotificationsSettings, OsdSettings, PowerMenuSettings, ScreenshotSettings,
    ScrollSettings, ScrollbarSettings, Section, SessionSettings, ShellFile, SpacesSettings,
    SwitcherSettings, WidgetsSettings, WindowSettings,
};

/// The table `S::KEY` names in `file`'s TOML, and whether it equals `S::of(file)`.
fn table_matches<S: Section + serde::Serialize>(file: &ShellFile) -> bool {
    let tree = toml::Value::try_from(file).unwrap_or_else(|e| panic!("{e}"));
    let table = tree.get(S::KEY).cloned();
    table == toml::Value::try_from(S::of(file)).ok()
}

/// A file whose every domain differs from the default, so a `Section` reading the wrong field
/// would be caught.
fn distinct() -> ShellFile {
    let mut file = ShellFile::default();
    file.bar.height_px = sill_settings::Px(31);
    file.dock.tile_size_px = sill_settings::Px(41);
    file.keyboard.overrides = vec![KeyboardOverride {
        app: "foot".to_owned(),
        profile: "term".to_owned(),
    }];
    file
}

#[test]
fn every_domain_finds_its_own_table() {
    let file = distinct();
    let cases: &[(&str, bool)] = &[
        ("bar", table_matches::<BarSettings>(&file)),
        ("calendar", table_matches::<CalendarSettings>(&file)),
        (
            "control_center",
            table_matches::<ControlCenterSettings>(&file),
        ),
        ("display", table_matches::<DisplaySettings>(&file)),
        ("dock", table_matches::<DockSettings>(&file)),
        ("hot_corners", table_matches::<HotCornersSettings>(&file)),
        ("idle", table_matches::<IdleSettings>(&file)),
        ("keyboard", table_matches::<KeyboardSettings>(&file)),
        ("launcher", table_matches::<LauncherSettings>(&file)),
        ("menus", table_matches::<MenusSettings>(&file)),
        (
            "notifications",
            table_matches::<NotificationsSettings>(&file),
        ),
        ("osd", table_matches::<OsdSettings>(&file)),
        ("power_menu", table_matches::<PowerMenuSettings>(&file)),
        ("screenshot", table_matches::<ScreenshotSettings>(&file)),
        ("scroll", table_matches::<ScrollSettings>(&file)),
        ("scrollbar", table_matches::<ScrollbarSettings>(&file)),
        ("session", table_matches::<SessionSettings>(&file)),
        ("spaces", table_matches::<SpacesSettings>(&file)),
        ("switcher", table_matches::<SwitcherSettings>(&file)),
        ("widgets", table_matches::<WidgetsSettings>(&file)),
        ("window", table_matches::<WindowSettings>(&file)),
    ];
    for (name, found) in cases {
        assert!(found, "{name}: KEY is not the table `of` reads");
    }
}

#[test]
fn a_section_reads_the_value_the_file_holds() {
    let file = distinct();
    assert_eq!(BarSettings::of(&file).height_px, sill_settings::Px(31));
    assert_eq!(DockSettings::of(&file).tile_size_px, sill_settings::Px(41));
    assert_ne!(
        BarSettings::of(&file),
        &BarSettings::default(),
        "the distinct file differs from the default"
    );
}
