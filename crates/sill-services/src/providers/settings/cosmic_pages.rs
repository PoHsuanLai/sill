//! The `cosmic-settings` page table, added only when that binary is on `PATH`.
//!
//! The page ids and names are `cosmic-settings`'s own subcommands (`cosmic-settings --help`,
//! version 1.7.0 on the machine this was written on: `cosmic-settings <page>` opens each one
//! directly); a static table rather than a discovered one, since `cosmic-settings` exposes no
//! machine-readable page list, and the set of pages changes only with a `cosmic-settings`
//! release.

use std::env;
use std::path::PathBuf;

use nucleo_matcher::Matcher;
use sill_launcher::{FrecencyScore, Needle, RankWeights, Score, rank_text};

/// `(page id, title, keywords)`.
const PAGES: &[(&str, &str, &[&str])] = &[
    ("accessibility", "Accessibility", &["a11y"]),
    (
        "accessibility-magnifier",
        "Accessibility Magnifier",
        &["zoom", "magnifier"],
    ),
    ("about", "About", &[]),
    (
        "appearance",
        "Appearance",
        &["theme", "colour", "color", "dark mode", "accent"],
    ),
    ("applications", "Applications", &[]),
    ("bluetooth", "Bluetooth", &[]),
    ("date-time", "Date & Time", &["clock", "timezone"]),
    ("default-apps", "Default Applications", &["default apps"]),
    ("desktop", "Desktop", &["wallpaper", "workspaces"]),
    ("displays", "Displays", &["monitor", "resolution", "scale"]),
    ("dock", "Dock", &[]),
    ("dock-applet", "Dock Applets", &[]),
    ("input", "Input Devices", &[]),
    ("keyboard", "Keyboard", &["shortcuts"]),
    ("legacy-applications", "Legacy Applications", &[]),
    ("mouse", "Mouse", &["pointer"]),
    ("network", "Network", &["wifi", "wi-fi", "ethernet"]),
    ("panel", "Panel", &["bar"]),
    ("panel-applet", "Panel Applets", &[]),
    ("power", "Power", &["battery"]),
    ("region-language", "Region & Language", &["locale"]),
    ("sound", "Sound", &["volume", "audio"]),
    ("startup-apps", "Startup Applications", &["autostart"]),
    ("system", "System & Accounts", &["users", "accounts"]),
    ("time", "Time & Language", &[]),
    ("touchpad", "Touchpad", &["gestures"]),
    ("users", "Users", &["accounts"]),
    ("vpn", "VPN", &[]),
    ("wallpaper", "Wallpaper", &["background"]),
    ("window-management", "Window Management", &["tiling"]),
    ("wired", "Wired", &["ethernet"]),
    ("wireless", "WiFi", &["wi-fi", "wireless"]),
    ("workspaces", "Workspaces", &[]),
];

/// The real `$PATH`, split into directories, as [`SettingsProvider::new`](super::SettingsProvider::new)
/// and [`SettingsProvider::with_dirs`](super::SettingsProvider::with_dirs) search it.
pub fn real_path_dirs() -> Vec<PathBuf> {
    env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default()
}

/// Whether `program` names a file directly under one of `dirs` — the search `$PATH` lookup
/// performs, but taking the directories as a plain argument so a test can supply a fake one
/// instead of the real `$PATH`: `std::env::set_var` is `unsafe` since edition 2024, and
/// CONVENTIONS §2 forbids `unsafe` outside shell-host's one allowed module.
pub fn is_on_path(dirs: &[PathBuf], program: &str) -> bool {
    dirs.iter().any(|dir| dir.join(program).is_file())
}

/// The static table, as [`super::SettingsProvider::cosmic_pages`] holds it.
pub fn cosmic_pages_table() -> Vec<(String, String, Vec<String>)> {
    PAGES
        .iter()
        .map(|&(id, title, keywords)| {
            (
                id.to_owned(),
                title.to_owned(),
                keywords.iter().map(|k| (*k).to_owned()).collect(),
            )
        })
        .collect()
}

/// Every page whose title or a keyword fuzzy-matches `needle`, as `(page id, title, rank)`.
pub fn search_cosmic_pages(
    pages: &[(String, String, Vec<String>)],
    needle: &Needle,
    matcher: &mut Matcher,
) -> Vec<(String, String, Score)> {
    let weights = RankWeights::DEFAULT;
    pages
        .iter()
        .filter_map(|(id, title, keywords)| {
            let candidates = std::iter::once(title).chain(keywords.iter());
            let best = candidates
                .filter_map(|text| rank_text(needle, text, FrecencyScore(0), &weights, matcher))
                .max()?;
            Some((id.clone(), title.clone(), best))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use nucleo_matcher::{Config, Matcher};
    use sill_launcher::Needle;

    use super::{cosmic_pages_table, search_cosmic_pages};

    #[test]
    fn the_table_has_no_duplicate_page_ids() {
        let table = cosmic_pages_table();
        let mut ids: Vec<&str> = table.iter().map(|(id, _, _)| id.as_str()).collect();
        let before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate page id in the COSMIC table");
    }

    #[test]
    fn a_title_match_finds_its_page() {
        let table = cosmic_pages_table();
        let mut m = Matcher::new(Config::DEFAULT);
        let found = search_cosmic_pages(&table, &Needle::parse("bluetooth"), &mut m);
        assert!(
            found.iter().any(|(id, _, _)| id == "bluetooth"),
            "{found:?}"
        );
    }

    #[test]
    fn a_keyword_match_finds_its_page() {
        let table = cosmic_pages_table();
        let mut m = Matcher::new(Config::DEFAULT);
        let found = search_cosmic_pages(&table, &Needle::parse("dark mode"), &mut m);
        assert!(
            found.iter().any(|(id, _, _)| id == "appearance"),
            "{found:?}"
        );
    }

    #[test]
    fn a_non_matching_query_finds_no_page() {
        let table = cosmic_pages_table();
        let mut m = Matcher::new(Config::DEFAULT);
        assert!(search_cosmic_pages(&table, &Needle::parse("zzzzzzzz"), &mut m).is_empty());
    }
}
