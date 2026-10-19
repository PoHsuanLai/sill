//! design/22-SETTINGS.md §9.6 for sill's schema: it round-trips, every kind has a widget, it
//! names sill's file, the Settings pages match §5, and the deep links resolve.

use ds_settings::schema::{Exposure, Page, Schema, deep_link_path};
use sill_settings::{schema, schemas};

#[test]
fn schema_round_trip() {
    for schema in schemas().into_iter().chain([schema()]) {
        let text = schema.to_toml();
        let back = Schema::from_toml(&text).unwrap_or_else(|e| panic!("{text}: {e}"));
        assert_eq!(back, schema, "{text}");
    }
}

#[test]
fn every_domain_names_sills_file() {
    for schema in schemas().into_iter().chain([schema()]) {
        assert_eq!(schema.app.0, "sill");
        assert_eq!(schema.file.0, "sill/settings.toml");
        assert_eq!(schema.version, 1);
    }
}

/// §5: the keys each Settings page shows. Every other sill key is Advanced (file only).
const BASIC: &[&str] = &[
    "dock.magnification",
    "dock.tile_size_px",
    "dock.autohide",
    "dock.autohide_delay_ms",
    "dock.autohide_slide_ms",
    "dock.indicators",
    "dock.bounce",
    "dock.launch_animation",
    "dock.click_active_app",
    "dock.trash",
    "scroll.natural",
    "notifications.dnd",
    "notifications.banner_style",
    "companion.summon",
    "companion.in_field",
    "companion.notify",
    "voice.hold_to_talk",
];

#[test]
fn exposure_follows_section_5() {
    for key in schema().key {
        let want = if BASIC.contains(&key.path.0.as_str()) {
            Exposure::Basic
        } else {
            Exposure::Advanced
        };
        assert_eq!(key.exposure, want, "{}", key.path.0);
    }
}

#[test]
fn basic_keys_are_on_their_section_5_pages() {
    let page_of = |prefix: &str| match prefix {
        "dock" => Page::Dock,
        "scroll" => Page::MouseAndGestures,
        "notifications" | "sound" => Page::Notifications,
        "spaces" => Page::Spaces,
        "companion" | "voice" => Page::Intelligence,
        other => panic!("no §5 page for {other}"),
    };
    for key in schema().key {
        if key.exposure == Exposure::Basic {
            let domain = key.path.0.split('.').next().unwrap_or_default().to_owned();
            assert_eq!(key.page, page_of(&domain), "{}", key.path.0);
        }
    }
}

/// §5: the Power page holds sill's idle service, every `idle.*` key and the lock grace,
/// all Advanced; nothing else is on it, and the rest of `session.*` stays on Accounts.
#[test]
fn the_power_page_is_idle_and_the_lock_grace() {
    let on_power = |path: &str| path.starts_with("idle.") || path == "session.lock_grace_s";
    let whole = schema();
    for key in &whole.key {
        let path = key.path.0.as_str();
        match (on_power(path), &key.page) {
            (true, Page::Power) => assert_eq!(key.exposure, Exposure::Advanced, "{path}"),
            (true, other) => panic!("{path} is on {other:?}, not Power"),
            (false, Page::Power) => panic!("{path} is on Power"),
            (false, _) => {}
        }
        if path.starts_with("session.") && !on_power(path) {
            assert_eq!(key.page, Page::Accounts, "{path}");
        }
    }
    assert_eq!(whole.key.iter().filter(|k| on_power(&k.path.0)).count(), 9);
}

/// The idle times carry their unit (`Secs` "s", `Mins` "min") and their §3.24 range.
#[test]
fn idle_times_are_bounded_in_their_units() {
    use ds_settings::schema::KeyKind;
    const CASES: &[(&str, i64, &str)] = &[
        ("session.lock_grace_s", 60, "s"),
        ("idle.dim_s", 60, "s"),
        ("idle.locked_screen_off_s", 600, "s"),
        ("idle.screen_off_ac_min", 240, "min"),
        ("idle.screen_off_battery_min", 240, "min"),
        ("idle.suspend_ac_min", 480, "min"),
        ("idle.suspend_battery_min", 480, "min"),
    ];
    let whole = schema();
    for &(path, max, unit) in CASES {
        let key = whole
            .key
            .iter()
            .find(|k| k.path.0 == path)
            .unwrap_or_else(|| panic!("{path} missing"));
        assert_eq!(
            key.kind,
            KeyKind::Bounded {
                min: 0,
                max,
                unit: Some(unit.to_owned())
            },
            "{path}"
        );
    }
}

#[test]
fn deep_links_resolve_through_the_schemas() {
    let all = schemas();
    const CASES: &[(&str, &str)] = &[
        ("magnification", "dock.magnification"),
        ("natural scrolling", "scroll.natural"),
        ("do not disturb", "notifications.dnd"),
        ("automatically hide", "dock.autohide"),
    ];
    for &(query, want) in CASES {
        assert_eq!(
            deep_link_path(&all, query).map(|p| p.0),
            Some(want.to_owned()),
            "{query:?}"
        );
    }
}

#[test]
fn defaults_in_the_schema_are_the_structs_defaults() {
    let whole = schema();
    let default_of = |path: &str| {
        whole
            .key
            .iter()
            .find(|k| k.path.0 == path)
            .map(|k| k.default.clone())
            .unwrap_or_else(|| panic!("{path} missing"))
    };
    let cases: &[(&str, toml::Value)] = &[
        ("bar.height_px", toml::Value::Integer(24)),
        ("dock.tile_size_px", toml::Value::Integer(48)),
        ("dock.influence_radius_px", toml::Value::Integer(72)),
        ("dock.magnification", toml::Value::String("on".to_owned())),
        ("dock.autohide", toml::Value::String("off".to_owned())),
        ("launcher.top_vertical_fraction", toml::Value::Integer(400)),
        ("scroll.natural", toml::Value::String("natural".to_owned())),
        ("scroll.momentum_a", toml::Value::Float(30.0)),
        ("notifications.banner_hold_ms", toml::Value::Integer(5200)),
        ("spaces.default_grain", toml::Value::Integer(0)),
        ("menus.first_mouse_window_ms", toml::Value::Integer(100)),
        (
            "display.hardware_volume",
            toml::Value::String("when_monitor_is_output".to_owned()),
        ),
        (
            "display.night_warmth",
            toml::Value::String("off".to_owned()),
        ),
        (
            "display.font_rendering",
            toml::Value::String("auto".to_owned()),
        ),
        ("osd.enabled", toml::Value::String("on".to_owned())),
        ("osd.hold_ms", toml::Value::Integer(1500)),
        ("osd.position", toml::Value::String("top_right".to_owned())),
        ("osd.margin_px", toml::Value::Integer(24)),
        ("widgets.desktop", toml::Value::String("off".to_owned())),
        ("widgets.desktop_cell_px", toml::Value::Integer(164)),
        ("calendar.sources", toml::Value::String("auto".to_owned())),
        (
            "calendar.first_weekday",
            toml::Value::String("monday".to_owned()),
        ),
        ("calendar.up_next_days", toml::Value::Integer(7)),
        ("screenshot.thumbnail_hold_ms", toml::Value::Integer(5200)),
        ("screenshot.tool", toml::Value::String("auto".to_owned())),
        (
            "hot_corners.top_left",
            toml::Value::String("none".to_owned()),
        ),
        ("hot_corners.dwell_ms", toml::Value::Integer(150)),
        ("hot_corners.size_px", toml::Value::Integer(2)),
        ("session.lock_grace_s", toml::Value::Integer(5)),
        ("idle.screen_off_ac_min", toml::Value::Integer(15)),
        ("idle.screen_off_battery_min", toml::Value::Integer(15)),
        ("idle.suspend_ac_min", toml::Value::Integer(30)),
        ("idle.suspend_battery_min", toml::Value::Integer(15)),
        ("idle.dim_s", toml::Value::Integer(5)),
        ("idle.dim_level_pct", toml::Value::Integer(50)),
        ("idle.lock", toml::Value::String("screen_off".to_owned())),
        ("idle.locked_screen_off_s", toml::Value::Integer(60)),
    ];
    for (path, want) in cases {
        assert_eq!(&default_of(path), want, "{path}");
    }
}
