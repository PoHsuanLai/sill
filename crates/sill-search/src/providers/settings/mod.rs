//! Settings pages and keys: quire and sill's own schemas (design/22 §9.3 "deep links").

pub mod index;
pub mod schema_files;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use nucleo_matcher::Matcher;
use sill_launcher::{
    Activation, Choice, IconRef, Item, ItemId, Provider, ProviderKind, Query, ResultSink, Score,
    SettingsTarget,
};

use ds_settings::schema::{Schema, data_dirs_from};
use index::{IndexedKey, SettingsIndex};
use schema_files::{fingerprint, schema_paths};

/// A key's `Item.id`: `activate` strips this back off, rather than re-parsing the row's title.
const KEY_ID_PREFIX: &str = "key:";
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
    /// Indexes the installed quire and sill schemas (`ds_settings::schema::data_dirs_from` of
    /// the data directory `data_home` and `$XDG_DATA_DIRS`, a system search path read like `$PATH`).
    pub fn new(data_home: &Path) -> SettingsProvider {
        SettingsProvider::with_dirs(data_dirs_from(
            Some(data_home.into()),
            std::env::var_os("XDG_DATA_DIRS"),
            None,
        ))
    }

    /// As [`SettingsProvider::new`], but discovering schemas under `dirs` instead of the real
    /// XDG directories — for tests and for a fixture install.
    pub fn with_dirs(dirs: Vec<PathBuf>) -> SettingsProvider {
        SettingsProvider {
            dirs,
            cached: Mutex::new(None),
        }
    }

    /// The current index, rebuilding it when the schema directory's contents have changed since
    /// the last query (a cheap file-listing fingerprint, not a full reparse, per query).
    fn index(&self) -> SettingsIndex {
        let paths = schema_paths(&self.dirs);
        let current = fingerprint(&paths);
        let mut guard = self.cached.lock().unwrap_or_else(PoisonError::into_inner);
        let stale = guard
            .as_ref()
            .is_none_or(|cached| cached.fingerprint != current);
        if stale {
            let schemas: Vec<Schema> = paths
                .iter()
                .filter_map(|path| {
                    let text = std::fs::read_to_string(path).ok()?;
                    match Schema::from_toml(&text) {
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
                index: SettingsIndex::build(&schemas),
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

        if query.is_cancelled() || items.is_empty() {
            return;
        }
        sink.push(items);
    }

    fn activate(&self, item: &Item, _choice: &Choice) -> Activation {
        if let Some(path) = item.id.0.strip_prefix(KEY_ID_PREFIX) {
            return Activation::OpenSettings {
                target: SettingsTarget::Key(path.to_owned()),
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
