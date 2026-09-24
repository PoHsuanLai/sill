//! How often and how recently a result was chosen (plan: "Frecency age buckets
//! 100/70/50/30/10 x count in $XDG_STATE_HOME/sill/frecency.json"). Pure: the file is read and
//! written by `sill-services`, the clock is an argument.

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
        let provider = match provider {
            ProviderKind::Apps => "apps",
            ProviderKind::Windows => "windows",
            ProviderKind::Calculator => "calculator",
            ProviderKind::Settings => "settings",
            ProviderKind::System => "system",
        };
        ItemKey(format!("{provider}:{}", item.0))
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

impl Frecency {
    /// `self` with one more use of `key` at `now`.
    pub fn record(mut self, key: ItemKey, now: UnixTime) -> Frecency {
        let entry = self.entries.entry(key).or_insert(Use {
            count: 0,
            last: now,
        });
        entry.count = entry.count.saturating_add(1);
        entry.last = entry.last.max(now);
        self
    }

    /// `key`'s frecency at `now`.
    pub fn score(&self, key: &ItemKey, now: UnixTime) -> FrecencyScore {
        self.entries.get(key).map_or(FrecencyScore::NONE, |used| {
            let raw = u64::from(age_bucket(now.days_since(used.last))) * u64::from(used.count);
            FrecencyScore(u16::try_from(raw.min(1000)).unwrap_or(1000))
        })
    }

    /// The file's contents; a damaged or foreign file is an empty history, never an error (a
    /// lost ranking hint is not worth a failed launcher).
    pub fn from_json(text: &str) -> Frecency {
        serde_json::from_str(text).unwrap_or_default()
    }

    /// The file's contents to write.
    pub fn to_json(&self) -> String {
        // Keys are strings and values are integers: nothing here can fail to encode.
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// The weight of a use made `days` ago. Buckets at 4, 14, 31 and 90 days (the plan names the
/// weights, not the boundaries; these are Firefox's frecency buckets, FINDINGS.md).
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
