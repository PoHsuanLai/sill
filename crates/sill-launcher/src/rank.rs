//! The launcher's ranking formula: score 0.7 match + 0.3 frecency + a boost for where the query
//! sits in the title (whole, prefix, word start), and a scattered hit scaled down.

pub mod sections;

use nucleo_matcher::Matcher;
use serde::{Deserialize, Serialize};

use crate::frecency::FrecencyScore;
use crate::matching::{MatchScore, Needle};

/// A result's rank; higher sorts first. Thousandths: a perfect match with full frecency and a
/// whole-title hit is `1000 + exact_boost`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Score(pub u32);

/// How the typed text sits in the title, best first: the launcher ranks a whole-title or
/// prefix hit above a hit at a word start, that above a plain substring, and all of those above
/// letters scattered along the title (`term` in "Capture Current Monitor"), which is what a
/// fuzzy matcher finds in nearly any long title.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Prefix {
    /// The letters are all there but spread along the title, or the title has them out of order
    /// for the matcher to find (`term` in "Capture Current Monitor").
    Scattered,
    /// The title contains the whole query inside a word: `mail` in "KMail".
    Elsewhere,
    /// Every typed word begins a word of the title: `mail` in "Check Mail".
    WordStarts,
    /// The title starts with the whole trimmed query.
    Starts,
    /// The title is the query.
    Exact,
}

impl Prefix {
    /// Where `query` sits in `title`, both lower-cased, the query trimmed. An empty query is
    /// [`Prefix::Elsewhere`]: nothing was typed, so no title is placed above another by it.
    pub fn of(query: &str, title: &str) -> Prefix {
        let query = query.trim().to_lowercase();
        let title = title.to_lowercase();
        let words_begin = || {
            query.split_whitespace().all(|word| {
                title
                    .split(|c: char| !c.is_alphanumeric())
                    .any(|part| part.starts_with(word))
            })
        };
        match () {
            () if query.is_empty() => Prefix::Elsewhere,
            () if title == query => Prefix::Exact,
            () if title.starts_with(&query) => Prefix::Starts,
            () if words_begin() => Prefix::WordStarts,
            () if title.contains(&query) => Prefix::Elsewhere,
            () => Prefix::Scattered,
        }
    }
}

/// The formula's weights, in thousandths. The plan fixes 0.7 and 0.3; it does not size the
/// boosts, so [`RankWeights::DEFAULT`] proposes them: a prefix hit +0.25, a whole-title hit
/// +0.35, a word-start hit +0.15, and a scattered hit worth 0.4 of its blend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankWeights {
    pub matched: u16,
    pub frecency: u16,
    pub exact_boost: u16,
    pub prefix_boost: u16,
    pub word_boost: u16,
    /// What a scattered hit's blend is multiplied by, in thousandths.
    pub scattered: u16,
}

impl RankWeights {
    /// 0.7 match, 0.3 frecency, the boosts above.
    pub const DEFAULT: RankWeights = RankWeights {
        matched: 700,
        frecency: 300,
        exact_boost: 350,
        prefix_boost: 250,
        word_boost: 150,
        scattered: 400,
    };
}

impl Default for RankWeights {
    fn default() -> Self {
        RankWeights::DEFAULT
    }
}

/// `matched x weight + frecency x weight`, plus the boost of where the query sits in the title
/// (a scattered hit's blend scaled down instead).
pub fn rank(
    matched: MatchScore,
    frecency: FrecencyScore,
    prefix: Prefix,
    weights: &RankWeights,
) -> Score {
    let blend = (u32::from(weights.matched) * u32::from(matched.0)
        + u32::from(weights.frecency) * u32::from(frecency.0))
        / 1000;
    let boost = |permille: u16| blend + u32::from(permille);
    Score(match prefix {
        Prefix::Exact => boost(weights.exact_boost),
        Prefix::Starts => boost(weights.prefix_boost),
        Prefix::WordStarts => boost(weights.word_boost),
        Prefix::Elsewhere => blend,
        Prefix::Scattered => blend * u32::from(weights.scattered) / 1000,
    })
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
            (1000, 1000, Prefix::WordStarts, 1150),
            (1000, 1000, Prefix::Starts, 1250),
            (1000, 1000, Prefix::Exact, 1350),
            (500, 500, Prefix::Elsewhere, 500),
            (800, 100, Prefix::Starts, 560 + 30 + 250),
            (1000, 0, Prefix::Scattered, 280),
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
    fn the_query_is_placed_in_the_title() {
        const CASES: &[(&str, &str, Prefix)] = &[
            ("fi", "Files", Prefix::Starts),
            ("FI", "files", Prefix::Starts),
            (" fi ", "Files", Prefix::Starts),
            ("fi", "Firefox", Prefix::Starts),
            ("files", "Files", Prefix::Exact),
            ("fox", "Firefox", Prefix::Elsewhere),
            ("mail", "KMail", Prefix::Elsewhere),
            ("mail", "Check Mail", Prefix::WordStarts),
            ("mail", "KMail Header Theme Editor", Prefix::Elsewhere),
            ("term", "Capture Current Monitor", Prefix::Scattered),
            ("term", "Telegram", Prefix::Scattered),
            ("", "Files", Prefix::Elsewhere),
            ("files manager", "Files", Prefix::Scattered),
            ("new win", "New Window", Prefix::Starts),
            ("win new", "New Window", Prefix::WordStarts),
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
