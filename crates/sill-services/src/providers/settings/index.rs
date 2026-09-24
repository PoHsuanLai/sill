//! The searchable index built from every discovered schema: pure once built, so ranking and
//! deep-link resolution are table-tested with no filesystem in sight.

use nucleo_matcher::Matcher;
use sill_launcher::{FrecencyScore, Needle, RankWeights, Score, rank_text};

use super::schema_file::{RawKey, RawSchema, page_label};

/// One settings key, flattened out of whichever schema named it (design/22 §9.3: "the query...
/// resolves... no hand-kept table" — every field here comes straight off the schema).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedKey {
    /// The dotted path (`"dock.magnification"`) — what `Activation::OpenSettings` needs.
    pub path: String,
    pub label: String,
    pub help: String,
    /// The page's display name, for the row's subtitle.
    pub page_label: String,
}

impl From<RawKey> for IndexedKey {
    fn from(raw: RawKey) -> IndexedKey {
        IndexedKey {
            path: raw.path,
            page_label: page_label(&raw.page),
            label: raw.label,
            help: raw.help,
        }
    }
}

/// Every indexed key from every schema found, in discovery order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsIndex {
    keys: Vec<IndexedKey>,
}

impl SettingsIndex {
    /// Flattens every schema's keys into one index, in the order the schemas were found.
    pub fn build(schemas: Vec<RawSchema>) -> SettingsIndex {
        SettingsIndex {
            keys: schemas
                .into_iter()
                .flat_map(|schema| schema.key)
                .map(IndexedKey::from)
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &IndexedKey> {
        self.keys.iter()
    }

    /// Every key whose path, label or help fuzzy-matches `needle`, with its rank (0.7 match,
    /// no frecency — a settings key has none of its own — plus the usual prefix boost).
    pub fn search(&self, needle: &Needle, matcher: &mut Matcher) -> Vec<(&IndexedKey, Score)> {
        let weights = RankWeights::DEFAULT;
        self.keys
            .iter()
            .filter_map(|key| {
                let candidates = [&key.label, &key.path, &key.help];
                let best = candidates
                    .into_iter()
                    .filter_map(|text| rank_text(needle, text, FrecencyScore(0), &weights, matcher))
                    .max()?;
                Some((key, best))
            })
            .collect()
    }

    /// The one key whose label or help mentions `query` (case-insensitive), or [`None`] when no
    /// key matches or more than one does (design/22 §9.3's deep link rule: "a query that
    /// resolves to two keys is not a deep link, it is a search result list").
    ///
    /// This mirrors `ds_settings::schema::deep_link`'s algorithm exactly; it is not that
    /// function, because `sill-services` cannot depend on `ds-settings`
    /// (`schema_file` module doc, `scripts/check-boundary.sh`).
    pub fn deep_link(&self, query: &str) -> Option<&IndexedKey> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return None;
        }
        let mut matches = self.keys.iter().filter(|key| {
            key.label.to_lowercase().contains(&query) || key.help.to_lowercase().contains(&query)
        });
        let first = matches.next()?;
        match matches.next() {
            None => Some(first),
            Some(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use nucleo_matcher::{Config, Matcher};
    use sill_launcher::Needle;

    use super::{IndexedKey, SettingsIndex};

    fn key(path: &str, label: &str, help: &str, page: &str) -> IndexedKey {
        IndexedKey {
            path: path.to_owned(),
            label: label.to_owned(),
            help: help.to_owned(),
            page_label: page.to_owned(),
        }
    }

    fn index() -> SettingsIndex {
        SettingsIndex {
            keys: vec![
                key(
                    "appearance.theme",
                    "Theme",
                    "Follow the desktop, or force light or dark mode.",
                    "Appearance",
                ),
                key(
                    "dock.magnification",
                    "Magnification",
                    "Icons grow as the pointer nears them.",
                    "Dock",
                ),
                key(
                    "scroll.natural",
                    "Natural scrolling",
                    "Content follows the finger, like a touchscreen.",
                    "Mouse & Gestures",
                ),
            ],
        }
    }

    #[test]
    fn a_unique_label_or_help_match_resolves_a_deep_link() {
        let idx = index();
        assert_eq!(
            idx.deep_link("dark mode").map(|k| k.path.as_str()),
            Some("appearance.theme")
        );
        assert_eq!(
            idx.deep_link("magnification").map(|k| k.path.as_str()),
            Some("dock.magnification")
        );
        assert_eq!(
            idx.deep_link("natural scrolling").map(|k| k.path.as_str()),
            Some("scroll.natural")
        );
    }

    #[test]
    fn a_query_matching_nothing_resolves_to_nothing() {
        assert_eq!(index().deep_link("bluetooth"), None);
    }

    #[test]
    fn an_ambiguous_query_resolves_to_nothing() {
        let idx = SettingsIndex {
            keys: vec![
                key("appearance.theme", "Theme", "light or dark", "Appearance"),
                key(
                    "appearance.look",
                    "Look",
                    "light or dark styling",
                    "Appearance",
                ),
            ],
        };
        assert_eq!(idx.deep_link("light"), None);
    }

    #[test]
    fn an_empty_query_has_no_deep_link() {
        assert_eq!(index().deep_link(""), None);
        assert_eq!(index().deep_link("   "), None);
    }

    #[test]
    fn search_finds_a_key_by_label_or_path() {
        let idx = index();
        let mut m = Matcher::new(Config::DEFAULT);
        let by_label = idx.search(&Needle::parse("magnif"), &mut m);
        assert_eq!(by_label.len(), 1);
        assert_eq!(by_label[0].0.path, "dock.magnification");

        let by_path = idx.search(&Needle::parse("dock.magnification"), &mut m);
        assert_eq!(by_path.len(), 1);
        assert_eq!(by_path[0].0.path, "dock.magnification");
    }

    #[test]
    fn a_non_matching_query_finds_nothing() {
        let idx = index();
        let mut m = Matcher::new(Config::DEFAULT);
        assert!(idx.search(&Needle::parse("zzzzzz"), &mut m).is_empty());
    }
}
