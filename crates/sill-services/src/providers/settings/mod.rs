//! Settings pages and keys: quire and sill's own schemas (design/22 §9.3 "deep links") and the
//! `cosmic-settings` page table when that binary exists.

pub mod cosmic_pages;
pub mod index;
pub mod schema_file;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use nucleo_matcher::Matcher;
use sill_launcher::{
    Activation, Batch, Choice, IconRef, Item, ItemId, Provider, ProviderKind, Query, ResultSink,
    Score, SettingsTarget,
};

use cosmic_pages::{cosmic_pages_table, is_on_path, real_path_dirs, search_cosmic_pages};
use index::{IndexedKey, SettingsIndex};
use schema_file::{data_dirs, fingerprint, parse_schema, schema_paths};

/// A key's `Item.id`: `activate` strips this back off, rather than re-parsing the row's title.
const KEY_ID_PREFIX: &str = "key:";
/// A COSMIC page's `Item.id`.
const COSMIC_ID_PREFIX: &str = "cosmic:";
/// A deep-linked key always outranks an ordinary fuzzy match within this provider's own rows.
const DEEP_LINK_SCORE: Score = Score(1000);

/// The last built index, and the directory fingerprint it was built from — so a query rebuilds
/// it only when the schema directory has actually changed (`SettingsProvider::index`).
#[derive(Debug)]
struct Cached {
    fingerprint: Vec<(PathBuf, u64, u64)>,
    index: SettingsIndex,
}

/// Rows for settings pages and keys matching the query.
#[derive(Debug)]
pub struct SettingsProvider {
    /// `(page id, title, keywords)` of `cosmic-settings` pages; empty when it is not installed.
    pub cosmic_pages: Vec<(String, String, Vec<String>)>,
    dirs: Vec<PathBuf>,
    cached: Mutex<Option<Cached>>,
}

impl Default for SettingsProvider {
    /// No schemas at all: rows can still be activated, none is ever found.
    fn default() -> SettingsProvider {
        SettingsProvider::with_dirs(Vec::new())
    }
}

impl SettingsProvider {
    /// Indexes the installed quire and sill schemas (`schema_file::data_dirs` of the data
    /// directory `data_home`), adding the COSMIC pages table when `cosmic-settings` is on `PATH`.
    pub fn new(data_home: &Path) -> SettingsProvider {
        SettingsProvider::with_dirs(data_dirs(data_home))
    }

    /// As [`SettingsProvider::new`], but discovering schemas under `dirs` instead of the real
    /// XDG directories — for tests and for a fixture install.
    pub fn with_dirs(dirs: Vec<PathBuf>) -> SettingsProvider {
        SettingsProvider::with_dirs_and_search_path(dirs, real_path_dirs())
    }

    /// As [`SettingsProvider::with_dirs`], but searching `search_path` for `cosmic-settings`
    /// instead of the real `$PATH` — the seam a test uses to prove the discovery without
    /// mutating the real environment (`cosmic_pages::is_on_path`'s doc says why that is
    /// unavailable here).
    fn with_dirs_and_search_path(
        dirs: Vec<PathBuf>,
        search_path: Vec<PathBuf>,
    ) -> SettingsProvider {
        SettingsProvider {
            cosmic_pages: if is_on_path(&search_path, "cosmic-settings") {
                cosmic_pages_table()
            } else {
                Vec::new()
            },
            dirs,
            cached: Mutex::new(None),
        }
    }

    /// The current index, rebuilding it when the schema directory's contents have changed since
    /// the last query (a cheap file-listing fingerprint, not a full reparse, per query).
    fn index(&self) -> SettingsIndex {
        let paths = schema_paths(&self.dirs);
        let current = fingerprint(&paths);
        let mut guard = self
            .cached
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let stale = guard
            .as_ref()
            .is_none_or(|cached| cached.fingerprint != current);
        if stale {
            let schemas = paths
                .iter()
                .filter_map(|path| {
                    let text = std::fs::read_to_string(path).ok()?;
                    match parse_schema(&text) {
                        Ok(schema) => Some(schema),
                        Err(error) => {
                            eprintln!(
                                "sill settings provider: skipping stale schema {}: {error}",
                                path.display()
                            );
                            None
                        }
                    }
                })
                .collect();
            *guard = Some(Cached {
                fingerprint: current,
                index: SettingsIndex::build(schemas),
            });
        }
        guard
            .as_ref()
            .map(|cached| cached.index.clone())
            .unwrap_or_default()
    }
}

impl Provider for SettingsProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Settings
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        if query.is_cancelled() {
            return;
        }
        let index = self.index();
        if query.is_cancelled() {
            return;
        }

        let mut matcher = Matcher::new(nucleo_matcher::Config::DEFAULT);
        let needle = sill_launcher::Needle::parse(&query.text);
        let mut items = Vec::new();

        let deep_linked = index.deep_link(&query.text);
        if let Some(key) = deep_linked {
            items.push(key_item(key, DEEP_LINK_SCORE));
        }
        for (key, score) in index.search(&needle, &mut matcher) {
            if deep_linked.is_some_and(|d| d.path == key.path) {
                continue;
            }
            items.push(key_item(key, score));
        }
        for (page_id, title, score) in
            search_cosmic_pages(&self.cosmic_pages, &needle, &mut matcher)
        {
            items.push(cosmic_item(&page_id, &title, score));
        }

        if query.is_cancelled() || items.is_empty() {
            return;
        }
        sink.push(Batch {
            provider: ProviderKind::Settings,
            generation: query.generation,
            items,
        });
    }

    fn activate(&self, item: &Item, _choice: &Choice) -> Activation {
        if let Some(path) = item.id.0.strip_prefix(KEY_ID_PREFIX) {
            return Activation::OpenSettings {
                target: SettingsTarget::Key(path.to_owned()),
            };
        }
        if let Some(page) = item.id.0.strip_prefix(COSMIC_ID_PREFIX) {
            return Activation::OpenSettings {
                target: SettingsTarget::Cosmic(page.to_owned()),
            };
        }
        Activation::Nothing
    }
}

fn key_item(key: &IndexedKey, score: Score) -> Item {
    Item {
        id: ItemId(format!("{KEY_ID_PREFIX}{}", key.path)),
        provider: ProviderKind::Settings,
        title: key.label.clone(),
        subtitle: key.page_label.clone(),
        icon: IconRef::Glyph("settings".to_owned()),
        score,
        actions: Vec::new(),
        subject: sill_launcher::Subject::None,
    }
}

fn cosmic_item(page_id: &str, title: &str, score: Score) -> Item {
    Item {
        id: ItemId(format!("{COSMIC_ID_PREFIX}{page_id}")),
        provider: ProviderKind::Settings,
        title: title.to_owned(),
        subtitle: "COSMIC Settings".to_owned(),
        icon: IconRef::Glyph("settings".to_owned()),
        score,
        actions: Vec::new(),
        subject: sill_launcher::Subject::None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::SettingsProvider;

    /// A fresh directory under the system temp dir, holding one file named `cosmic-settings` —
    /// standing in for a `$PATH` entry, removed on drop (no `tempfile`: not in the pinned block,
    /// `results/test_support.rs` uses the same pattern).
    struct FakePathDir(PathBuf);

    impl FakePathDir {
        fn with_cosmic_settings() -> FakePathDir {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "sill-services-settings-fake-path-{}-{n}",
                std::process::id()
            ));
            fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
            fs::write(dir.join("cosmic-settings"), b"").unwrap_or_else(|e| panic!("{e}"));
            FakePathDir(dir)
        }
    }

    impl Drop for FakePathDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn cosmic_pages_appear_when_cosmic_settings_is_on_the_search_path() {
        let fake = FakePathDir::with_cosmic_settings();
        let provider =
            SettingsProvider::with_dirs_and_search_path(Vec::new(), vec![fake.0.clone()]);
        assert!(
            !provider.cosmic_pages.is_empty(),
            "{:?}",
            provider.cosmic_pages
        );
    }

    #[test]
    fn no_cosmic_pages_when_cosmic_settings_is_not_on_the_search_path() {
        let provider = SettingsProvider::with_dirs_and_search_path(Vec::new(), Vec::new());
        assert!(
            provider.cosmic_pages.is_empty(),
            "{:?}",
            provider.cosmic_pages
        );
    }
}
