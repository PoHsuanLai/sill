//! The emoji the launcher searches: every fully-qualified emoji the `emojis` crate knows
//! (Unicode's emoji-test data, its CLDR short name, group and skin-tone variants; MIT OR
//! Apache-2.0 AND Unicode-3.0), each with its CLDR English keywords from the bundled
//! `data/emoji-keywords.tsv` (Unicode-3.0, `scripts/gen-emoji-keywords.sh`). Lane c fills it.
//!
//! The data model says nothing about painting: a row carries the emoji as text and the system's
//! colour font draws it (Noto Color Emoji, COLRv1, here). quire's `AnimatedEmoji` catalogue is
//! a separate, animated subset for the user's picture and is not used here.

use std::collections::HashMap;

/// The keyword table, bundled at build time.
pub const KEYWORDS_TSV: &str = include_str!("../../../data/emoji-keywords.tsv");

/// Whether an emoji takes a skin tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tones {
    Takes,
    Fixed,
}

/// One searchable emoji.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmojiEntry {
    /// The emoji, default (yellow) tone.
    pub glyph: &'static str,
    /// CLDR short name, lower case.
    pub name: &'static str,
    /// CLDR keywords (the name's words not repeated), lower case.
    pub keywords: Vec<&'static str>,
    /// GitHub shortcodes (`grinning`, `smiley`), for a `:` query.
    pub shortcodes: Vec<&'static str>,
    pub tones: Tones,
}

/// `KEYWORDS_TSV` as a map from emoji to its keywords: `#` lines and blank lines skipped, each
/// line `<emoji>\t<kw>|<kw>|…`.
pub fn parse_keywords(tsv: &'static str) -> HashMap<&'static str, Vec<&'static str>> {
    let _ = tsv;
    todo!("M9 fill: c-emoji-web-units")
}

/// Every emoji, in Unicode's order (the grid's order with nothing typed and nothing recent),
/// built once.
pub fn emoji_table() -> &'static [EmojiEntry] {
    todo!("M9 fill: c-emoji-web-units")
}

#[cfg(test)]
mod tests {
    use super::{KEYWORDS_TSV, Tones, emoji_table, parse_keywords};

    #[test]
    fn the_bundled_table_is_unicode_licensed_and_non_empty() {
        assert!(KEYWORDS_TSV.contains("SPDX-License-Identifier: Unicode-3.0"));
        assert!(KEYWORDS_TSV.lines().filter(|l| !l.starts_with('#')).count() > 1000);
    }

    #[test]
    #[ignore = "M9 fill: c-emoji-web-units"]
    fn keywords_and_names_come_through() {
        let keywords = parse_keywords(KEYWORDS_TSV);
        assert!(keywords["😀"].contains(&"grin"));
        let table = emoji_table();
        let wave = table.iter().find(|e| e.glyph == "👋").expect("waving hand");
        assert_eq!(wave.name, "waving hand");
        assert_eq!(wave.tones, Tones::Takes);
        let grin = table
            .iter()
            .find(|e| e.glyph == "😀")
            .expect("grinning face");
        assert_eq!(grin.tones, Tones::Fixed);
        assert!(grin.keywords.contains(&"smile"));
    }
}
