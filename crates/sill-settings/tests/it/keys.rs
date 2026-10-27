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
    // Settings keeps only what macOS offers (2026-10-27): these sizes are stylesheet tokens
    // (`--dock-gap`, `--cc-width`, `--bar-status-box`, ...) or constants beside their readers.
    // design/22 still has their rows; they are quire's to remove.
    "bar.height_px",
    "bar.status_icon_box_px",
    "bar.status_glyph_px",
    "dock.tile_gap_px",
    "dock.pill_radius_px",
    "dock.edge_clamp_px",
    "dock.running_dot_diameter_px",
    "dock.separator_height_px",
    "dock.pill_padding_px",
    "dock.running_dot_gap_px",
    "launcher.top_bar_gap_px",
    "launcher.top_centre_bias_px",
    "scrollbar.track_thickness_px",
    "scrollbar.track_thickness_hover_px",
    "scrollbar.thumb_thickness_px",
    "scrollbar.thumb_thickness_hover_px",
    "scrollbar.thumb_min_length_px",
    "scrollbar.legacy_track_width_px",
    "scrollbar.other_axis_offset_px",
    "osd.margin_px",
    "notifications.position_right_px",
    "notifications.position_top_gap_px",
    "notifications.banner_width_px",
    "notifications.icon_px",
    "notifications.center_width_px",
    "control_center.width_px",
    "control_center.grid_gap_px",
    "control_center.grid_padding_px",
    "control_center.bottom_margin_px",
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
    assert!(keys.len() > 60, "only {} keys parsed", keys.len());
    for key in keys.iter().filter(|key| !RETIRED.contains(&key.as_str())) {
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
