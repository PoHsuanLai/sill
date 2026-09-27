//! design/22-SETTINGS.md §6 test 5 and §9.6 `every_key_in_catalogue_has_a_spec`, for sill's
//! domains: every key §3.4-3.8 and §3.11-3.14 name (plus the three §3.1 rows sill owns) has a
//! `KeySpec`, and every `KeySpec` has a row, path for path.

mod common;

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
    ("### 3.14 `spaces`", "## 4. Rust shape"),
];

/// §3.1 rows that are sill's, not `AppearanceSettings`': the mirror image of ds-settings'
/// `SILL_OWNED` list (its `tests/keys.rs`, quire FINDINGS F15).
const FROM_APPEARANCE: &[&str] = &[
    "notifications.banner_material",
    "control_center.material",
    "launcher.material",
];

/// Rows that are aliases of another file's key, not keys of their own: §3.14 says
/// `spaces.material_tint_alpha` is "alias of `appearance.material_tint_alpha` (3.1) kept in this
/// domain's table for discoverability". The mirror of ds-settings' `DOC_ALIASES` (`motion.level`).
const DOC_ALIASES: &[&str] = &["spaces.material_tint_alpha"];

/// Keys registered in sill whose §3 rows quire's design/22 has not gained yet: quire's owner adds
/// them (sill FINDINGS lists each row). A key here needs no row until it has one; once the row
/// lands it is checked like every other key, and the entry can go.
const AWAITING_ROWS: &[&str] = &[];

/// Keys sill has dropped whose §3 rows quire's design/22 still carries: the mirror of
/// [`AWAITING_ROWS`] for a retirement (design/28 §6.0). sill drops the spec and lists the key
/// here, quire's owner removes the row, then the entry goes. A key here needs no spec while its
/// row remains. (`dock.shipped_icons` went this way: quire fd77336 retired its §3.5 row, F1043.)
const RETIRING: &[&str] = &[];

fn doc_keys() -> Vec<String> {
    let doc = doc();
    let mut keys: Vec<String> = SECTIONS
        .iter()
        .flat_map(|(from, to)| keys_in_section(&doc, from, to))
        .filter(|key| !DOC_ALIASES.contains(&key.as_str()))
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
    for key in keys.iter().filter(|key| !RETIRING.contains(&key.as_str())) {
        assert!(specs.contains(key), "{key}: no KeySpec with that path");
    }
}

#[test]
fn every_retiring_key_is_unregistered() {
    let specs = spec_paths();
    for path in RETIRING {
        assert!(
            !specs.iter().any(|s| s == path),
            "{path} is retiring but still has a KeySpec"
        );
    }
}

#[test]
fn every_spec_has_a_row_in_the_catalogue() {
    let keys = doc_keys();
    let awaited = |path: &str| AWAITING_ROWS.contains(&path) && !keys.iter().any(|k| k == path);
    for path in spec_paths().into_iter().filter(|path| !awaited(path)) {
        assert!(
            keys.contains(&path),
            "{path} has a KeySpec but no row in 22-SETTINGS.md"
        );
    }
}

#[test]
fn every_key_awaiting_its_row_is_registered() {
    let specs = spec_paths();
    for path in AWAITING_ROWS {
        assert!(
            specs.iter().any(|s| s == path),
            "{path} awaits a row but has no KeySpec"
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

#[test]
fn every_alias_row_named_here_is_still_in_the_doc() {
    let doc = doc();
    let spaces = keys_in_section(&doc, "### 3.14 `spaces`", "## 4. Rust shape");
    for alias in DOC_ALIASES {
        assert!(
            spaces.iter().any(|k| k == alias),
            "{alias} is not a §3.14 row"
        );
    }
}
