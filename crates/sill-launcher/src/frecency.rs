//! How often and how recently a result was chosen (age buckets 100/70/50/30/10 x count, in
//! `$XDG_STATE_HOME/sill/frecency.json`). Pure: the file is read and written by `sill-services`,
//! the clock is an argument.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{ItemId, ProviderKind};
use crate::time::UnixTime;

/// Which result an entry counts: `<provider>:<item id>`, e.g. `apps:org.mozilla.firefox`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemKey(pub String);

impl ItemKey {
    /// The key for `item` from `provider`.
    pub fn of(provider: ProviderKind, item: &ItemId) -> ItemKey {
        ItemKey(format!("{}:{}", key_prefix(provider), item.0))
    }

    /// The provider this key was made for by [`ItemKey::of`], read back from its own prefix
    /// (the format is this module's, so this is not a guess at someone else's text).
    pub fn provider(&self) -> Option<ProviderKind> {
        let (prefix, _) = self.0.split_once(':')?;
        ProviderKind::ALL
            .into_iter()
            .find(|kind| key_prefix(*kind) == prefix)
    }

    /// Whether choosing this key's row is counted: its provider's [`counting`]; a key of no known
    /// provider is counted.
    pub fn counting(&self) -> Counting {
        self.provider().map_or(Counting::Counted, counting)
    }
}

/// Each provider's prefix in `frecency.json`.
fn key_prefix(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Apps => "apps",
        ProviderKind::Windows => "windows",
        ProviderKind::Calculator => "calculator",
        ProviderKind::Settings => "settings",
        ProviderKind::System => "system",
        ProviderKind::Files => "files",
        ProviderKind::Clipboard => "clipboard",
        ProviderKind::Emoji => "emoji",
        ProviderKind::Web => "web",
        ProviderKind::Things => "things",
        ProviderKind::Companion => "companion",
        ProviderKind::Runs => "runs",
        ProviderKind::Memory => "memory",
    }
}

/// How often and when last a result was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Use {
    pub count: u32,
    pub last: UnixTime,
}

/// A result's frecency in thousandths: `age bucket x count`, capped at 1000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrecencyScore(pub u16);

impl FrecencyScore {
    /// Never chosen.
    pub const NONE: FrecencyScore = FrecencyScore(0);
}

/// Every counted result: the whole of `frecency.json`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Frecency {
    #[serde(default)]
    pub entries: BTreeMap<ItemKey, Use>,
}

/// The most results the history keeps. Years of use name a few thousand different results at
/// most; past this the ones not used for longest go, so the file and the cost of every choice
/// stay bounded.
pub const MAX_ENTRIES: usize = 2000;

impl Frecency {
    /// `self` with one more use of `key` at `now`, the history kept within [`MAX_ENTRIES`] by
    /// dropping the results used least recently.
    pub fn record(mut self, key: ItemKey, now: UnixTime) -> Frecency {
        let entry = self.entries.entry(key).or_insert(Use {
            count: 0,
            last: now,
        });
        entry.count = entry.count.saturating_add(1);
        entry.last = entry.last.max(now);
        self.pruned()
    }

    /// `self` within [`MAX_ENTRIES`]: the results used least recently are dropped first.
    fn pruned(mut self) -> Frecency {
        let over = self.entries.len().saturating_sub(MAX_ENTRIES);
        if over > 0 {
            let mut by_age: Vec<(UnixTime, ItemKey)> = self
                .entries
                .iter()
                .map(|(key, used)| (used.last, key.clone()))
                .collect();
            by_age.sort();
            for (_, key) in by_age.into_iter().take(over) {
                self.entries.remove(&key);
            }
        }
        self
    }

    /// `key`'s frecency at `now`.
    pub fn score(&self, key: &ItemKey, now: UnixTime) -> FrecencyScore {
        self.entries.get(key).map_or(FrecencyScore::NONE, |used| {
            let raw = u64::from(age_bucket(now.days_since(used.last))) * u64::from(used.count);
            FrecencyScore(u16::try_from(raw.min(1000)).unwrap_or(1000))
        })
    }

    /// The file's contents, within [`MAX_ENTRIES`]; an error when they are not a history.
    pub fn parse(text: &str) -> Result<Frecency, serde_json::Error> {
        serde_json::from_str::<Frecency>(text).map(Frecency::pruned)
    }

    /// The file's contents; a damaged or foreign file is an empty history, never an error (a
    /// lost ranking hint is not worth a failed launcher).
    pub fn from_json(text: &str) -> Frecency {
        Frecency::parse(text).unwrap_or_default()
    }

    /// The file's contents to write.
    pub fn to_json(&self) -> String {
        // Keys are strings and values are integers: nothing here can fail to encode.
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Whether choosing a provider's row is counted in `frecency.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Counting {
    Counted,
    /// Never written to disk: a clipboard entry's key means nothing after the session, and the
    /// history must leave no trace of its use.
    NotCounted,
}

/// Whether `provider`'s rows are counted. The web fallback is counted under its fixed id (never
/// the query, which is not a row's identity).
pub fn counting(provider: ProviderKind) -> Counting {
    match provider {
        // An answer, a run and a memory row name something that ends or is forgotten; choosing
        // one is not a habit to rank by.
        ProviderKind::Clipboard
        | ProviderKind::Companion
        | ProviderKind::Runs
        | ProviderKind::Memory => Counting::NotCounted,
        // What a person calculated is in the row's id, and it must not be kept in a file; a
        // result is not a habit to rank by either.
        ProviderKind::Calculator => Counting::NotCounted,
        ProviderKind::Apps
        | ProviderKind::Windows
        | ProviderKind::Settings
        | ProviderKind::System
        | ProviderKind::Files
        | ProviderKind::Emoji
        | ProviderKind::Things
        | ProviderKind::Web => Counting::Counted,
    }
}

/// The weight of a use made `days` ago. Buckets at 4, 14, 31 and 90 days (the plan names the
/// weights, not the boundaries; these are Firefox's frecency buckets).
pub fn age_bucket(days: u64) -> u16 {
    match days {
        0..=4 => 100,
        5..=14 => 70,
        15..=31 => 50,
        32..=90 => 30,
        _ => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::{Frecency, FrecencyScore, ItemKey, Use, age_bucket};
    use crate::ids::{ItemId, ProviderKind};
    use crate::time::UnixTime;

    const DAY: u64 = 86_400;
    const NOW: UnixTime = UnixTime(1_790_000_000);

    #[test]
    fn a_key_knows_its_provider_and_clipboard_keys_are_never_counted() {
        use super::Counting;
        for kind in ProviderKind::ALL {
            let key = ItemKey::of(kind, &ItemId("a:b".to_owned()));
            assert_eq!(key.provider(), Some(kind), "{key:?}");
        }
        let clip = ItemKey::of(ProviderKind::Clipboard, &ItemId("7".to_owned()));
        assert_eq!(clip.counting(), Counting::NotCounted);
        let sum = ItemKey::of(ProviderKind::Calculator, &ItemId("calc:42".to_owned()));
        assert_eq!(
            sum.counting(),
            Counting::NotCounted,
            "what was calculated is not kept"
        );
        let app = ItemKey::of(ProviderKind::Apps, &ItemId("foot".to_owned()));
        assert_eq!(app.counting(), Counting::Counted);
        assert_eq!(ItemKey("odd".to_owned()).counting(), Counting::Counted);
    }

    #[test]
    fn the_history_stays_within_its_cap_by_dropping_the_least_recently_used() {
        use super::MAX_ENTRIES;
        let mut history = Frecency::default();
        for n in 0..=MAX_ENTRIES {
            history = history.record(key(&format!("app-{n}")), UnixTime(NOW.0 + n as u64));
        }
        assert_eq!(history.entries.len(), MAX_ENTRIES);
        assert!(
            !history.entries.contains_key(&key("app-0")),
            "the oldest went"
        );
        assert!(
            history
                .entries
                .contains_key(&key(&format!("app-{MAX_ENTRIES}")))
        );
        let big = serde_json::to_string(&Frecency {
            entries: (0..MAX_ENTRIES + 10)
                .map(|n| {
                    (
                        key(&format!("a{n}")),
                        Use {
                            count: 1,
                            last: UnixTime(n as u64),
                        },
                    )
                })
                .collect(),
        })
        .unwrap_or_default();
        assert_eq!(
            Frecency::parse(&big).map(|f| f.entries.len()).ok(),
            Some(MAX_ENTRIES),
            "a file longer than the cap is cut when it is read"
        );
    }

    fn key(id: &str) -> ItemKey {
        ItemKey::of(ProviderKind::Apps, &ItemId(id.to_owned()))
    }

    fn days_ago(days: u64) -> UnixTime {
        UnixTime(NOW.0 - days * DAY)
    }

    #[test]
    fn age_buckets_are_100_70_50_30_10() {
        const CASES: &[(u64, u16)] = &[
            (0, 100),
            (4, 100),
            (5, 70),
            (14, 70),
            (15, 50),
            (31, 50),
            (32, 30),
            (90, 30),
            (91, 10),
            (10_000, 10),
        ];
        for &(days, want) in CASES {
            assert_eq!(age_bucket(days), want, "{days} days");
        }
    }

    #[test]
    fn score_is_bucket_times_count_capped_at_1000() {
        const CASES: &[(u32, u64, u16)] = &[
            (1, 0, 100),
            (3, 0, 300),
            (3, 10, 210),
            (2, 20, 100),
            (4, 60, 120),
            (5, 365, 50),
            (10, 0, 1000),
            (500, 0, 1000),
        ];
        for &(count, age, want) in CASES {
            let mut frecency = Frecency::default();
            frecency.entries.insert(
                key("foot"),
                Use {
                    count,
                    last: days_ago(age),
                },
            );
            assert_eq!(
                frecency.score(&key("foot"), NOW),
                FrecencyScore(want),
                "{count} uses, {age} days ago"
            );
        }
    }

    #[test]
    fn an_unknown_key_scores_nothing() {
        let frecency = Frecency::default().record(key("foot"), NOW);
        assert_eq!(frecency.score(&key("firefox"), NOW), FrecencyScore::NONE);
    }

    #[test]
    fn recording_counts_up_and_moves_last_forward_only() {
        let once = Frecency::default().record(key("foot"), days_ago(10));
        assert_eq!(once.score(&key("foot"), NOW), FrecencyScore(70));
        let twice = once.record(key("foot"), NOW);
        assert_eq!(
            twice.entries.get(&key("foot")),
            Some(&Use {
                count: 2,
                last: NOW
            })
        );
        // A use stamped earlier (a clock that stepped back) never ages the entry.
        let thrice = twice.record(key("foot"), days_ago(100));
        assert_eq!(
            thrice.entries.get(&key("foot")),
            Some(&Use {
                count: 3,
                last: NOW
            })
        );
    }

    #[test]
    fn a_future_use_counts_as_today() {
        let frecency = Frecency::default().record(key("foot"), UnixTime(NOW.0 + 5 * DAY));
        assert_eq!(frecency.score(&key("foot"), NOW), FrecencyScore(100));
    }

    #[test]
    fn the_file_round_trips_and_damage_is_an_empty_history() {
        let frecency = Frecency::default()
            .record(key("foot"), NOW)
            .record(key("org.mozilla.firefox"), days_ago(3));
        assert_eq!(Frecency::from_json(&frecency.to_json()), frecency);
        for text in [
            "",
            "not json",
            "[1,2]",
            r#"{"entries":{"apps:x":{"count":"many"}}}"#,
        ] {
            assert_eq!(Frecency::from_json(text), Frecency::default(), "{text:?}");
        }
    }

    #[test]
    fn keys_carry_their_provider() {
        assert_eq!(
            ItemKey::of(ProviderKind::Windows, &ItemId("42".to_owned())),
            ItemKey("windows:42".to_owned())
        );
    }
}
