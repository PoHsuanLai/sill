//! Fuzzy matching with nucleo, smart case (plan: "nucleo Pattern smart-case"), normalised so a
//! score means the same thing whatever the query's length.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use serde::{Deserialize, Serialize};

/// How well a haystack matches a needle, in thousandths of the needle's own best score
/// (the score it gets against its own text). 1000 is as good as an exact match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MatchScore(pub u16);

impl MatchScore {
    /// What every haystack scores against an empty query: ranking falls to frecency alone.
    pub const NONE: MatchScore = MatchScore(0);
    /// As good as the needle's own text.
    pub const FULL: MatchScore = MatchScore(1000);
}

/// A parsed query, ready to score haystacks. Parsed once per keystroke, used for every row.
#[derive(Debug)]
pub struct Needle {
    text: String,
    pattern: Pattern,
    /// The needle's score against its own text: the normalisation denominator. `None` when the
    /// needle cannot match itself (a negated atom such as `!foo`).
    best: Option<u32>,
}

impl Needle {
    /// Parse `text` with nucleo's pattern syntax (`^`, `$`, `'`, `!`), smart case and smart
    /// Unicode normalisation.
    pub fn parse(text: &str) -> Needle {
        let pattern = Pattern::parse(text, CaseMatching::Smart, Normalization::Smart);
        let mut buf = Vec::new();
        let best = pattern.score(Utf32Str::new(text, &mut buf), &mut matcher());
        Needle {
            text: text.to_owned(),
            pattern,
            best,
        }
    }

    /// The text as typed.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether nothing was typed (whitespace only counts as nothing).
    pub fn is_empty(&self) -> bool {
        self.pattern.atoms.is_empty()
    }

    /// `haystack`'s score, or `None` when it does not match. An empty needle matches everything
    /// with [`MatchScore::NONE`].
    pub fn score(&self, haystack: &str, matcher: &mut Matcher) -> Option<MatchScore> {
        if self.is_empty() {
            return Some(MatchScore::NONE);
        }
        let mut buf = Vec::new();
        let raw = self
            .pattern
            .score(Utf32Str::new(haystack, &mut buf), matcher)?;
        Some(normalise(raw, self.best))
    }
}

/// A matcher with nucleo's default (non-path) configuration. One per thread; reuse it.
pub fn matcher() -> Matcher {
    Matcher::new(Config::DEFAULT)
}

/// `raw` in thousandths of `best`, capped at [`MatchScore::FULL`].
fn normalise(raw: u32, best: Option<u32>) -> MatchScore {
    match best {
        Some(best) if best > 0 => {
            let permille = u64::from(raw) * 1000 / u64::from(best);
            MatchScore(u16::try_from(permille.min(1000)).unwrap_or(1000))
        }
        _ => MatchScore::FULL,
    }
}

#[cfg(test)]
mod tests {
    use super::{MatchScore, Needle, matcher, normalise};

    #[test]
    fn the_needles_own_text_scores_full() {
        let mut m = matcher();
        for text in ["firefox", "Fire", "fire fox", "ter"] {
            assert_eq!(
                Needle::parse(text).score(text, &mut m),
                Some(MatchScore::FULL),
                "{text}"
            );
        }
    }

    #[test]
    fn scores_order_exact_before_prefix_before_scattered() {
        let mut m = matcher();
        let needle = Needle::parse("term");
        let score = |haystack: &str, m: &mut _| needle.score(haystack, m);
        let exact = score("term", &mut m);
        let prefix = score("terminal", &mut m);
        let scattered = score("the rust macro", &mut m);
        assert!(exact >= prefix, "{exact:?} {prefix:?}");
        assert!(prefix > scattered, "{prefix:?} {scattered:?}");
        assert!(scattered.is_some(), "a subsequence still matches");
    }

    #[test]
    fn a_non_match_is_none() {
        let mut m = matcher();
        const CASES: &[(&str, &str)] = &[("xyz", "firefox"), ("foxfire", "firefox")];
        for &(needle, haystack) in CASES {
            assert_eq!(
                Needle::parse(needle).score(haystack, &mut m),
                None,
                "{needle} in {haystack}"
            );
        }
    }

    #[test]
    fn case_is_smart() {
        let mut m = matcher();
        // Lower-case needles ignore case; a capital makes the needle case-sensitive.
        assert!(Needle::parse("files").score("Files", &mut m).is_some());
        assert!(Needle::parse("Files").score("files", &mut m).is_none());
        assert!(Needle::parse("Files").score("Files", &mut m).is_some());
    }

    #[test]
    fn an_empty_needle_matches_everything_with_no_score() {
        let mut m = matcher();
        for text in ["", "   "] {
            let needle = Needle::parse(text);
            assert!(needle.is_empty(), "{text:?}");
            assert_eq!(needle.score("anything", &mut m), Some(MatchScore::NONE));
        }
    }

    #[test]
    fn normalisation_is_thousandths_of_best_capped() {
        const CASES: &[(u32, Option<u32>, u16)] = &[
            (50, Some(100), 500),
            (100, Some(100), 1000),
            (150, Some(100), 1000),
            (0, Some(100), 0),
            (7, None, 1000),
            (7, Some(0), 1000),
        ];
        for &(raw, best, want) in CASES {
            assert_eq!(normalise(raw, best), MatchScore(want), "{raw} of {best:?}");
        }
    }
}
