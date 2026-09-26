//! The launcher's ranking formula (plan: "score 0.7 match + 0.3 frecency + prefix boost").

pub mod sections;

use nucleo_matcher::Matcher;
use serde::{Deserialize, Serialize};

use crate::frecency::FrecencyScore;
use crate::matching::{MatchScore, Needle};

/// A result's rank; higher sorts first. Thousandths: a perfect match with full frecency and a
/// prefix boost is `1000 + prefix_boost`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Score(pub u32);

/// Whether the title starts with the typed text (case-insensitively), which earns the boost:
/// typing `fi` should put Files above Firefox's fuzzier cousins only when Files starts with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Prefix {
    /// The title starts with the whole trimmed query.
    Starts,
    /// It does not, or the query is empty.
    Elsewhere,
}

impl Prefix {
    /// Whether `title` starts with `query`, both lower-cased, the query trimmed.
    pub fn of(query: &str, title: &str) -> Prefix {
        let query = query.trim().to_lowercase();
        if !query.is_empty() && title.to_lowercase().starts_with(&query) {
            Prefix::Starts
        } else {
            Prefix::Elsewhere
        }
    }
}

/// The formula's weights, in thousandths. The plan fixes 0.7 and 0.3; it does not size the
/// prefix boost, so [`RankWeights::DEFAULT`] proposes 0.1 (FINDINGS.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankWeights {
    pub matched: u16,
    pub frecency: u16,
    pub prefix_boost: u16,
}

impl RankWeights {
    /// 0.7 match, 0.3 frecency, +0.1 prefix.
    pub const DEFAULT: RankWeights = RankWeights {
        matched: 700,
        frecency: 300,
        prefix_boost: 100,
    };
}

impl Default for RankWeights {
    fn default() -> Self {
        RankWeights::DEFAULT
    }
}

/// `matched x weight + frecency x weight + boost when the title starts with the query`.
pub fn rank(
    matched: MatchScore,
    frecency: FrecencyScore,
    prefix: Prefix,
    weights: &RankWeights,
) -> Score {
    let blend = (u32::from(weights.matched) * u32::from(matched.0)
        + u32::from(weights.frecency) * u32::from(frecency.0))
        / 1000;
    let boost = match prefix {
        Prefix::Starts => u32::from(weights.prefix_boost),
        Prefix::Elsewhere => 0,
    };
    Score(blend + boost)
}

/// The rank of a row titled `title` for `needle`, or `None` when it does not match.
pub fn rank_text(
    needle: &Needle,
    title: &str,
    frecency: FrecencyScore,
    weights: &RankWeights,
    matcher: &mut Matcher,
) -> Option<Score> {
    let matched = needle.score(title, matcher)?;
    Some(rank(
        matched,
        frecency,
        Prefix::of(needle.text(), title),
        weights,
    ))
}

#[cfg(test)]
mod tests {
    use super::{Prefix, RankWeights, Score, rank, rank_text};
    use crate::frecency::FrecencyScore;
    use crate::matching::{MatchScore, Needle, matcher};

    #[test]
    fn the_formula_is_seven_tenths_match_three_tenths_frecency_plus_the_boost() {
        const CASES: &[(u16, u16, Prefix, u32)] = &[
            (0, 0, Prefix::Elsewhere, 0),
            (1000, 0, Prefix::Elsewhere, 700),
            (0, 1000, Prefix::Elsewhere, 300),
            (1000, 1000, Prefix::Elsewhere, 1000),
            (1000, 1000, Prefix::Starts, 1100),
            (500, 500, Prefix::Elsewhere, 500),
            (800, 100, Prefix::Starts, 560 + 30 + 100),
            (1, 1, Prefix::Elsewhere, 1),
        ];
        for &(m, f, prefix, want) in CASES {
            assert_eq!(
                rank(
                    MatchScore(m),
                    FrecencyScore(f),
                    prefix,
                    &RankWeights::DEFAULT
                ),
                Score(want),
                "m {m} f {f} {prefix:?}"
            );
        }
    }

    #[test]
    fn prefix_is_whole_query_at_the_start_ignoring_case() {
        const CASES: &[(&str, &str, Prefix)] = &[
            ("fi", "Files", Prefix::Starts),
            ("FI", "files", Prefix::Starts),
            (" fi ", "Files", Prefix::Starts),
            ("fi", "Firefox", Prefix::Starts),
            ("fox", "Firefox", Prefix::Elsewhere),
            ("", "Files", Prefix::Elsewhere),
            ("files manager", "Files", Prefix::Elsewhere),
        ];
        for &(query, title, want) in CASES {
            assert_eq!(Prefix::of(query, title), want, "{query:?} / {title}");
        }
    }

    #[test]
    fn frecency_breaks_a_tie_between_equal_matches() {
        let mut m = matcher();
        let needle = Needle::parse("term");
        let w = RankWeights::DEFAULT;
        let used = rank_text(&needle, "Terminal", FrecencyScore(700), &w, &mut m);
        let unused = rank_text(&needle, "Terminal", FrecencyScore(0), &w, &mut m);
        assert!(used > unused, "{used:?} {unused:?}");
    }

    #[test]
    fn a_non_match_has_no_rank_whatever_its_frecency() {
        let mut m = matcher();
        let needle = Needle::parse("zzz");
        assert_eq!(
            rank_text(
                &needle,
                "Terminal",
                FrecencyScore(1000),
                &RankWeights::DEFAULT,
                &mut m
            ),
            None
        );
    }

    #[test]
    fn an_empty_query_ranks_by_frecency_alone() {
        let mut m = matcher();
        let needle = Needle::parse("");
        let w = RankWeights::DEFAULT;
        assert_eq!(
            rank_text(&needle, "Files", FrecencyScore(1000), &w, &mut m),
            Some(Score(300))
        );
        assert_eq!(
            rank_text(&needle, "Files", FrecencyScore(0), &w, &mut m),
            Some(Score(0))
        );
    }
}
