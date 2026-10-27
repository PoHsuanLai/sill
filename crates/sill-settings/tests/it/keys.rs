//! design/22-SETTINGS.md §6 test 5 and §9.6 `every_key_in_catalogue_has_a_spec`, for sill's
//! domains: every key §3.4-3.8 and §3.11-3.14 name (plus the §3.1 rows sill owns) has a
//! `KeySpec`, and every `KeySpec` has a row, path for path.

use crate::common;

use common::{doc, keys_in_section};
use sill_settings::schemas;

/// The sections that are sill's, each with the heading that ends it. §3.9 `gestures` and §3.10
/// `palm_rejection` are palmrest's file, not this one.
const SECTIONS: &[(&str, &str)] = &[
    ("### 3.4 `bar`", "### 3.5 `dock`"),
    ("### 3.5 `dock`", "### 3.6 `launcher`"),
    ("### 3.6 `launcher`", "### 3.7 `scroll`"),
    ("### 3.7 `scroll`", "### 3.8 `scrollbar`"),
    ("### 3.8 `scrollbar`", "### 3.9 `gestures`"),
    ("### 3.11 `menus`", "### 3.12 `notifications`"),
    ("### 3.12 `notifications`", "### 3.13 `control_center`"),
    ("### 3.13 `control_center`", "### 3.14 `spaces`"),
    ("### 3.14 `spaces`", "### 3.26 `ai`"),
    ("### 3.30 `companion`", "### 3.31 `viewer`"),
    ("### 3.33 `wallpaper`", "## 4. Rust shape"),
];

/// §3.1 rows that are sill's, not `AppearanceSettings`': the mirror image of ds-settings'
/// `SILL_OWNED` list (its `tests/keys.rs`).
const FROM_APPEARANCE: &[&str] = &[];

/// Keys sill ships that design/22 has no row for yet; the row is quire's to add (FINDINGS: open
/// items), and this list ends when it does. `bar.autohide` is the Mac's "Automatically hide and
/// show the menu bar".
const UNCATALOGUED: &[&str] = &["voice.hold_to_talk", "bar.autohide"];

/// Rows design/22 still has for keys sill dropped (the widget settings, 2026-10-20: the center's
/// widgets are fixed and there is no desktop layer); the rows are quire's to remove, and this
/// list ends when they go.
const RETIRED: &[&str] = &[
    "widgets.center",
    "widgets.desktop",
    "widgets.desktop_widgets",
    "widgets.desktop_cell_px",
    "widgets.desktop_gap_px",
    "widgets.world_clocks",
];

/// Rows design/22 still has for keys sill turned into constants or style (2026-10-27: Settings
/// shows only what the OS offers); the rows are quire's to remove, and this list ends when they go.
const PRUNED: &[&str] = &[
    "bar.title_padding_px",
    "bar.open_title_pill_height_px",
    "bar.status_gap_px",
    "bar.glyph_size_policy",
    "bar.item_font_px",
    "bar.item_font_weight",
    "bar.item_radius_px",
    "dock.influence_radius_px",
    "dock.autohide_delay_ms",
    "dock.autohide_slide_ms",
    "dock.autohide_trigger_strip_px",
    "dock.click_active_app",
    "dock.magnify_enter_ms",
    "dock.magnify_leave_ms",
    "dock.hover_label_warm_ms",
    "dock.hold_to_menu_ms",
    "dock.hold_to_menu_move_px",
    "dock.remove_threshold_px",
    "dock.spring_load_ms",
    "dock.context_menu_order",
    "dock.modifier_clicks",
    "launcher.top_vertical_fraction",
    "launcher.field_font_px",
    "launcher.field_font_weight",
    "launcher.field_glyph_px",
    "launcher.row_title_px",
    "launcher.row_detail_px",
    "launcher.files_backend",
    "launcher.files_budget_ms",
    "launcher.clipboard_cap",
    "launcher.clipboard_image_max_mb",
    "launcher.web_engine_url",
    "scroll.momentum_model",
    "scroll.momentum_a",
    "scroll.momentum_b",
    "scroll.momentum_stop_px_s",
    "scroll.momentum_start_px_s",
    "scroll.momentum_cap_px_s",
    "scroll.velocity_window_ms",
    "scroll.velocity_held_still_ms",
    "scroll.rubber_band",
    "scroll.rubber_band_c",
    "scroll.rubber_band_linear_divisor",
    "scroll.rubber_band_edge_min_px",
    "scroll.rubber_band_snap_rate",
    "scroll.rubber_band_snap_gain",
    "scroll.wheel_detent_px",
    "scroll.wheel_burst_window_ms",
    "scroll.momentum_boost_threshold_px_s",
    "scroll.double_scroll_grace_ms",
    "scrollbar.hide_delay_ms",
    "scrollbar.fade_duration_ms",
    "scrollbar.expand_zone_px",
    "scrollbar.expand_duration_ms",
    "menus.submenu_delay_ms",
    "menus.item_height_px",
    "menus.separator_margin_px",
    "menus.submenu_triangle_timeout_ms",
    "menus.first_mouse_window_ms",
    "menus.font_px",
    "menus.highlight_radius_px",
    "menus.tooltip_font_px",
    "notifications.banner_min_height_px",
    "notifications.banner_padding_px",
    "notifications.banner_hold_ms",
    "notifications.hover_min_remaining_ms",
    "notifications.close_button_px",
    "notifications.swipe_dismiss_px",
    "notifications.swipe_dismiss_velocity_px_s",
    "notifications.swipe_damping",
    "notifications.stack_max",
    "notifications.stack_gap_px",
    "notifications.group_offset_px",
    "notifications.server",
    "notifications.banner_position",
    "notifications.history_cap",
    "control_center.grid_columns",
    "spaces.default_grain",
    "osd.enabled",
    "osd.hold_ms",
    "osd.position",
    "window.move_threshold_px",
    "power_menu.default_action",
    "power_menu.material",
    "calendar.up_next_days",
    "calendar.up_next_max",
    "screenshot.thumbnail_hold_ms",
    "screenshot.tool",
    "idle.dim_s",
    "idle.dim_level_pct",
    "idle.locked_screen_off_s",
    "companion.double_tap_ms",
    "companion.app_wait_ms",
    "companion.confirm_arm_ms",
    "launcher.material",
    "notifications.banner_material",
];

fn doc_keys() -> Vec<String> {
    let doc = doc();
    let mut keys: Vec<String> = SECTIONS
        .iter()
        .flat_map(|(from, to)| keys_in_section(&doc, from, to))
        .collect();
    let appearance = keys_in_section(&doc, "### 3.1 `appearance`", "### 3.2 `motion`");
    for key in FROM_APPEARANCE {
        assert!(
            appearance.iter().any(|k| k == key),
            "{key} is listed as sill's but is not a §3.1 row any more"
        );
        keys.push((*key).to_owned());
    }
    keys
}

fn spec_paths() -> Vec<String> {
    schemas()
        .iter()
        .flat_map(|schema| schema.key.iter().map(|key| key.path.0.clone()))
        .collect()
}

#[test]
fn every_key_in_catalogue_has_a_spec() {
    let specs = spec_paths();
    let keys = doc_keys();
    assert!(keys.len() > 100, "only {} keys parsed", keys.len());
    for key in keys
        .iter()
        .filter(|key| !RETIRED.contains(&key.as_str()) && !PRUNED.contains(&key.as_str()))
    {
        assert!(specs.contains(key), "{key}: no KeySpec with that path");
    }
}

#[test]
fn every_spec_has_a_row_in_the_catalogue() {
    let keys = doc_keys();
    for path in spec_paths()
        .into_iter()
        .filter(|path| !UNCATALOGUED.contains(&path.as_str()))
    {
        assert!(
            keys.contains(&path),
            "{path} has a KeySpec but no row in 22-SETTINGS.md"
        );
    }
}

#[test]
fn no_path_is_described_twice() {
    let mut paths = spec_paths();
    let count = paths.len();
    paths.sort();
    paths.dedup();
    assert_eq!(paths.len(), count, "a key path appears in two domains");
}
