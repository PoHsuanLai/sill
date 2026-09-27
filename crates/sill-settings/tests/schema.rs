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
fn widget_for_kind_is_total() {
    let whole = schema();
    assert!(!whole.key.is_empty());
    for key in &whole.key {
        let _ = key.kind.widget();
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
    "dock.position",
    "dock.indicators",
    "dock.bounce",
    "dock.launch_animation",
    "dock.click_active_app",
    "dock.trash",
    "scroll.natural",
    "notifications.dnd",
    "notifications.banner_style",
    "sound.ui_sounds",
    "sound.volume_feedback",
    "spaces.mail_frame_policy",
    "spaces.wallpaper_follows_space",
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
        other => panic!("no §5 page for {other}"),
    };
    for key in schema().key {
        if key.exposure == Exposure::Basic {
            let domain = key.path.0.split('.').next().unwrap_or_default().to_owned();
            assert_eq!(key.page, page_of(&domain), "{}", key.path.0);
        }
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
        ("bar.height_px", toml::Value::Integer(32)),
        ("dock.tile_size_px", toml::Value::Integer(48)),
        ("dock.influence_radius_px", toml::Value::Integer(96)),
        ("dock.magnification", toml::Value::String("on".to_owned())),
        ("dock.autohide", toml::Value::String("off".to_owned())),
        ("launcher.top_vertical_fraction", toml::Value::Integer(400)),
        ("scroll.natural", toml::Value::String("natural".to_owned())),
        ("scroll.momentum_a", toml::Value::Float(30.0)),
        ("notifications.banner_hold_ms", toml::Value::Integer(5200)),
        ("sound.theme", toml::Value::String("freedesktop".to_owned())),
        ("spaces.default_grain", toml::Value::Integer(40)),
        ("menus.first_mouse_window_ms", toml::Value::Integer(100)),
        ("display.scale", toml::Value::String("auto".to_owned())),
        ("display.density_target_ppi", toml::Value::Integer(109)),
        (
            "display.scale_rounding",
            toml::Value::String("prefer_whole".to_owned()),
        ),
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
        ("session.locker", toml::Value::String("sill".to_owned())),
        (
            "session.polkit_agent",
            toml::Value::String("auto".to_owned()),
        ),
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
    ];
    for (path, want) in cases {
        assert_eq!(&default_of(path), want, "{path}");
    }
}
