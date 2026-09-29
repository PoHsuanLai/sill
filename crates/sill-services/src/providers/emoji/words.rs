//! Word-prefix matching for emoji. Pure.
//!
//! A fuzzy score alone lets a query match letters scattered across a long name: "lantern" is a
//! subsequence of "flag: Netherlands". Spotlight lists an emoji only when each typed word begins
//! a word of its name or keywords, so a flag needs "flag" or its country's word. Words split on
//! anything that is not a letter or a digit ("jack-o-lantern" has the word "lantern") and compare
//! without case.

/// Whether a query's words each begin a word of an emoji's fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Words {
    /// Every typed word is the prefix of some word in the fields (or nothing typed is a word).
    Prefixed,
    /// Some typed word begins no word there: its letters are scattered.
    Scattered,
}

/// How `needle`'s words stand against the words of `fields`.
pub fn words_in<'a>(needle: &str, fields: impl IntoIterator<Item = &'a str>) -> Words {
    let have: Vec<String> = fields.into_iter().flat_map(words).collect();
    let prefixed = words(needle).all(|typed| have.iter().any(|word| word.starts_with(&typed)));
    match prefixed {
        true => Words::Prefixed,
        false => Words::Scattered,
    }
}

/// `text`'s words, lower-cased.
fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
}

#[cfg(test)]
mod tests {
    use super::{Words, words_in};

    #[test]
    fn each_typed_word_must_begin_a_word() {
        let cases: &[(&str, &[&str], Words)] = &[
            ("lantern", &["red paper lantern"], Words::Prefixed),
            ("lantern", &["jack-o-lantern"], Words::Prefixed),
            ("lantern", &["flag: Netherlands"], Words::Scattered),
            (
                "lantern",
                &["flag: Northern Mariana Islands"],
                Words::Scattered,
            ),
            ("thumbs", &["thumbs up"], Words::Prefixed),
            ("Thumbs U", &["thumbs up"], Words::Prefixed),
            ("up thumbs", &["thumbs up"], Words::Prefixed),
            ("humbs", &["thumbs up"], Words::Scattered),
            // Words may come from different fields: the name and a keyword.
            ("red lamp", &["red paper lantern", "lamp"], Words::Prefixed),
            ("tada", &["party popper", "tada"], Words::Prefixed),
            ("neth", &["flag: Netherlands"], Words::Prefixed),
            // Nothing that is a word: the fuzzy score alone decides.
            ("+", &["thumbs up"], Words::Prefixed),
        ];
        for &(needle, fields, want) in cases {
            assert_eq!(
                words_in(needle, fields.iter().copied()),
                want,
                "{needle} in {fields:?}"
            );
        }
    }
}
