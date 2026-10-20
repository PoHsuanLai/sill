//! The searchable index built from every discovered schema: pure once built, so ranking and
//! deep-link resolution are table-tested with no filesystem in sight.

use nucleo_matcher::Matcher;
use sill_launcher::{FrecencyScore, Needle, RankWeights, Score, rank_text};

use ds_settings::schema::{KeySpec, PageId, PageTable, Schema};

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
    /// The id of the key's page: what opening that page names.
    pub page: PageId,
}

impl IndexedKey {
    /// `spec` as `table` names its page: the declared id and title, else the page's own.
    fn of(spec: &KeySpec, table: &PageTable) -> IndexedKey {
        IndexedKey {
            path: spec.path.0.clone(),
            page_label: table.title_of(&spec.page).0,
            page: table
                .resolve(spec.page.id().as_str())
                .unwrap_or_else(|| spec.page.id()),
            label: spec.label.0.clone(),
            help: spec.help.0.clone(),
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
    pub fn build(schemas: &[Schema]) -> SettingsIndex {
        let table = PageTable::of_schemas(schemas);
        SettingsIndex {
            keys: schemas
                .iter()
                .flat_map(|schema| &schema.key)
                .map(|spec| IndexedKey::of(spec, &table))
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
    /// The rule of `ds_settings::schema::deep_link` over the flattened keys, with the query
    /// trimmed and an empty query resolving to nothing.
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
    use ds_settings::schema::{PageId, Schema};

    fn key(path: &str, label: &str, help: &str, page: &str) -> IndexedKey {
        IndexedKey {
            path: path.to_owned(),
            label: label.to_owned(),
            help: help.to_owned(),
            page_label: page.to_owned(),
            page: PageId::new(&page.to_lowercase()),
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

    #[test]
    fn a_schema_file_indexes_each_key_with_its_pages_label() {
        let text = r#"
app = "sill"
file = "sill/settings.toml"
version = 1

[[key]]
path = "dock.magnification"
default = "on"
label = "Magnification"
help = "Icons grow as the pointer nears them."
section = "Magnification"
exposure = "basic"

[key.kind]
kind = "toggle"

[key.kind.v]
variants = ["on", "off"]

[key.page]
kind = "dock"

[[key]]
path = "idle.lock_after_mins"
default = "5"
label = "Lock after"
help = "Minutes of idleness before the screen locks."
section = "Idle"
exposure = "basic"

[key.kind]
kind = "text"

[key.page]
kind = "power"
"#;
        let schema = Schema::from_toml(text).expect("the schema text parses");
        let index = SettingsIndex::build(&[schema]);
        let keys: Vec<(&str, &str, &str, &str)> = index
            .iter()
            .map(|key| {
                (
                    key.path.as_str(),
                    key.label.as_str(),
                    key.help.as_str(),
                    key.page_label.as_str(),
                )
            })
            .collect();
        assert_eq!(
            keys,
            vec![
                (
                    "dock.magnification",
                    "Magnification",
                    "Icons grow as the pointer nears them.",
                    "Desktop & Dock"
                ),
                (
                    "idle.lock_after_mins",
                    "Lock after",
                    "Minutes of idleness before the screen locks.",
                    "Power"
                ),
            ]
        );
    }

    #[test]
    fn a_malformed_schema_file_does_not_parse() {
        assert!(Schema::from_toml("not = [valid").is_err());
    }
}
