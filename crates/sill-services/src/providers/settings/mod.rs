//! Settings pages and keys: quire and sill's own schemas (design/22 §9.3 "deep links") and the
//! `cosmic-settings` page table when that binary exists.

pub mod cosmic_pages;
pub mod index;
pub mod schema_file;

use std::path::PathBuf;
use std::sync::Mutex;

use nucleo_matcher::Matcher;
use sill_launcher::{
    Activation, Batch, Choice, IconRef, Item, ItemId, Provider, ProviderKind, Query, ResultSink,
    Score, SettingsTarget,
};

use cosmic_pages::{cosmic_pages_table, cosmic_settings_available, search_cosmic_pages};
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
    fn default() -> SettingsProvider {
        SettingsProvider::new()
    }
}

impl SettingsProvider {
    /// Indexes the installed quire and sill schemas (`schema_file::data_dirs`, the real XDG
    /// directories), adding the COSMIC pages table when `cosmic-settings` is on `PATH`.
    pub fn new() -> SettingsProvider {
        SettingsProvider::with_dirs(data_dirs())
    }

    /// As [`SettingsProvider::new`], but discovering schemas under `dirs` instead of the real
    /// XDG directories — for tests and for a fixture install.
    pub fn with_dirs(dirs: Vec<PathBuf>) -> SettingsProvider {
        SettingsProvider {
            cosmic_pages: if cosmic_settings_available() {
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
    }
}
