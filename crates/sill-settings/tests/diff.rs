//! design/22-SETTINGS.md §6 test 4, sill's half: two files differing in one domain yield that
//! domain only; identical files yield nothing.

use ds_settings::{Count, Ms, Px};
use sill_settings::{
    Dnd, PanelMaterial, PickFeedback, SettingsChange, ShellFile, TrackClick, UiSounds, apply,
};

fn with(edit: impl FnOnce(&mut ShellFile)) -> ShellFile {
    let mut file = ShellFile::default();
    edit(&mut file);
    file
}

#[test]
fn apply_yields_exactly_the_changed_domain() {
    let cases: Vec<(ShellFile, SettingsChange)> = vec![
        (with(|f| f.bar.height_px = Px(24)), SettingsChange::Bar),
        (with(|f| f.dock.tile_size_px = Px(64)), SettingsChange::Dock),
        (
            with(|f| f.launcher.material = PanelMaterial::Popover),
            SettingsChange::Launcher,
        ),
        (
            with(|f| f.scroll.wheel_detent_px = Px(53)),
            SettingsChange::Scroll,
        ),
        (
            with(|f| f.scrollbar.track_click_mode = TrackClick::JumpToClick),
            SettingsChange::Scrollbar,
        ),
        (
            with(|f| f.menus.pick_feedback = PickFeedback::BlinkOnce),
            SettingsChange::Menus,
        ),
        (
            with(|f| f.switcher.show_delay_ms = Ms(0)),
            SettingsChange::Switcher,
        ),
        (
            with(|f| f.sound.ui_sounds = UiSounds::Off),
            SettingsChange::Sound,
        ),
        (
            with(|f| f.notifications.dnd = Dnd::On),
            SettingsChange::Notifications,
        ),
        (
            with(|f| f.control_center.grid_columns = Count(3)),
            SettingsChange::ControlCenter,
        ),
        (
            with(|f| f.spaces.default_grain = Count(10)),
            SettingsChange::Spaces,
        ),
    ];
    let base = ShellFile::default();
    for (changed, want) in cases {
        assert_eq!(apply(&base, &changed), vec![want], "{want:?}");
        assert_eq!(apply(&changed, &base), vec![want], "{want:?} reversed");
    }
}

#[test]
fn identical_files_and_unknown_tables_change_nothing() {
    let base = ShellFile::default();
    assert!(apply(&base, &base.clone()).is_empty());
    let extra = with(|f| {
        f.extra.insert("later".to_owned(), toml::Value::Integer(1));
        f.version = 2;
    });
    assert_eq!(apply(&base, &extra), Vec::<SettingsChange>::new());
}

#[test]
fn several_domains_come_in_table_order() {
    let base = ShellFile::default();
    let many = with(|f| {
        f.spaces.default_grain = Count(1);
        f.bar.height_px = Px(30);
        f.sound.ui_sounds = UiSounds::Off;
    });
    assert_eq!(
        apply(&base, &many),
        vec![
            SettingsChange::Bar,
            SettingsChange::Sound,
            SettingsChange::Spaces
        ]
    );
}
