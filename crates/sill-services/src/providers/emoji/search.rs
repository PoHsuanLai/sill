//! Matching emoji. Pure; lane c fills it.
//!
//! Typed text matches the short name first (a whole-word hit on the name ranks above a
//! keyword), then keywords, then shortcodes (`:smile` with a leading colon searches shortcodes
//! and names only), each only when every typed word begins a word there (`words.rs`, Q350: no
//! letters scattered across a long name); nothing typed with the Emoji filter lists the recent ones (the runner's
//! frecency, `emoji:<glyph>` keys) then the rest in Unicode order: Spotlight's recent-first
//! grid. The skin tone applies to every emoji that takes one; the frecency key is always the
//! default-tone glyph, so changing the tone keeps the recents.

use nucleo_matcher::Matcher;
use sill_launcher::{
    EmojiFacts, Frecency, IconRef, Item, ItemId, ItemKey, MatchScore, Needle, ProviderKind, Score,
    SkinTone, Subject, UnixTime,
};

use super::table::EmojiEntry;
use super::words::{Words, words_in};

/// Room for every table position in a score's low digits, comfortably above the table's length
/// (~1900 default-tone emoji), so a tie above it never falls back to a title sort.
const POSITION_SPACE: u32 = 10_000;

/// Which of an entry's fields answered a typed query: the short name first, then its keywords,
/// then its shortcodes (this module's doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchKind {
    Shortcode,
    Keyword,
    Name,
}

/// `table`'s emoji as rows for `needle`, best first, at most `limit`.
pub fn emoji_items(
    table: &[EmojiEntry],
    needle: &Needle,
    tone: SkinTone,
    frecency: &Frecency,
    now: UnixTime,
    limit: usize,
    matcher: &mut Matcher,
) -> Vec<Item> {
    if needle.is_empty() {
        return recent_first(table, tone, frecency, now, limit);
    }
    let raw = needle.text().trim_start();
    let shortcode_needle = raw.strip_prefix(':').and_then(|rest| {
        let rest = rest.trim_start();
        (!rest.is_empty()).then(|| Needle::parse(rest))
    });
    let shortcode_only = shortcode_needle.is_some();
    let active = shortcode_needle.as_ref().unwrap_or(needle);

    let mut ranked: Vec<(Score, &EmojiEntry)> = table
        .iter()
        .enumerate()
        .filter(|(_, entry)| searched_words(entry, active, shortcode_only) == Words::Prefixed)
        .filter_map(|(position, entry)| {
            let (kind, score) = best_match(entry, active, shortcode_only, matcher)?;
            Some((text_score(kind, score, position), entry))
        })
        .collect();
    ranked.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    ranked
        .into_iter()
        .take(limit)
        .map(|(score, entry)| to_item(entry, tone, score))
        .collect()
}

/// Whether each of `needle`'s words begins a word of the fields a search reads: the name, the
/// keywords (not after `:`) and the shortcodes (Q350: no scattered letters).
fn searched_words(entry: &EmojiEntry, needle: &Needle, shortcode_only: bool) -> Words {
    let keywords: &[&str] = match shortcode_only {
        true => &[],
        false => &entry.keywords,
    };
    let fields = std::iter::once(entry.name)
        .chain(keywords.iter().copied())
        .chain(entry.shortcodes.iter().copied());
    words_in(needle.text(), fields)
}

/// `entry`'s best match for `needle`: its name, then (outside a `:` search) its keywords, then
/// its shortcodes, the first of these that matches at all.
fn best_match(
    entry: &EmojiEntry,
    needle: &Needle,
    shortcode_only: bool,
    matcher: &mut Matcher,
) -> Option<(MatchKind, MatchScore)> {
    if let Some(score) = needle.score(entry.name, matcher) {
        return Some((MatchKind::Name, score));
    }
    if !shortcode_only
        && let Some(score) = entry
            .keywords
            .iter()
            .filter_map(|keyword| needle.score(keyword, matcher))
            .max()
    {
        return Some((MatchKind::Keyword, score));
    }
    entry
        .shortcodes
        .iter()
        .filter_map(|shortcode| needle.score(shortcode, matcher))
        .max()
        .map(|score| (MatchKind::Shortcode, score))
}

/// A typed-text match's score: the fuzzy score dominates (so a strong keyword hit is never
/// beaten by a scattered, low-quality name match), the kind breaks a tie in the name's favour,
/// and the table's own order breaks what is left.
fn text_score(kind: MatchKind, score: MatchScore, position: usize) -> Score {
    let kind_rank = match kind {
        MatchKind::Name => 2,
        MatchKind::Keyword => 1,
        MatchKind::Shortcode => 0,
    };
    Score(u32::from(score.0) * 100_000 + kind_rank * POSITION_SPACE + reverse_position(position))
}

/// Nothing typed: the runner's frecency leads (`emoji:<glyph>` keys, the default-tone glyph),
/// then the table's own order (Spotlight's recent-first grid).
fn recent_first(
    table: &[EmojiEntry],
    tone: SkinTone,
    frecency: &Frecency,
    now: UnixTime,
    limit: usize,
) -> Vec<Item> {
    let mut ranked: Vec<(Score, &EmojiEntry)> = table
        .iter()
        .enumerate()
        .map(|(position, entry)| {
            let key = ItemKey::of(ProviderKind::Emoji, &ItemId(entry.glyph.to_owned()));
            let used = frecency.score(&key, now);
            let score = Score(u32::from(used.0) * POSITION_SPACE + reverse_position(position));
            (score, entry)
        })
        .collect();
    ranked.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    ranked
        .into_iter()
        .take(limit)
        .map(|(score, entry)| to_item(entry, tone, score))
        .collect()
}

/// `position` as a low digit that favours an earlier (Unicode-ordered) entry; the table is far
/// short of [`POSITION_SPACE`], so this never goes negative in spirit (it saturates instead).
fn reverse_position(position: usize) -> u32 {
    let position = u32::try_from(position).unwrap_or(u32::MAX);
    POSITION_SPACE.saturating_sub(position)
}

/// One row for `entry`: the row's title is the glyph itself (`tone` applied), its subtitle the
/// CLDR name; the frecency key is always the default-tone glyph (F650), so changing the tone
/// keeps the recents.
fn to_item(entry: &EmojiEntry, tone: SkinTone, score: Score) -> Item {
    let glyph = toned(entry.glyph, tone);
    Item {
        id: ItemId(entry.glyph.to_owned()),
        provider: ProviderKind::Emoji,
        title: glyph.clone(),
        subtitle: entry.name.to_owned(),
        icon: IconRef::None,
        score,
        actions: Vec::new(),
        subject: Subject::Emoji(EmojiFacts {
            glyph,
            name: entry.name.to_owned(),
            tone,
        }),
    }
}

/// `glyph` with `tone` applied (Unicode's modifier after the base, inside a ZWJ sequence where
/// Unicode puts it), or as it is when it takes none or `tone` is the default.
pub fn toned(glyph: &str, tone: SkinTone) -> String {
    if tone == SkinTone::Default {
        return glyph.to_owned();
    }
    emojis::get(glyph)
        .and_then(|emoji| emoji.with_skin_tone(emojis_tone(tone)))
        .map(|emoji| emoji.as_str().to_owned())
        .unwrap_or_else(|| glyph.to_owned())
}

/// This crate's [`SkinTone`] as the `emojis` crate's own enum (the same six, Default first).
fn emojis_tone(tone: SkinTone) -> emojis::SkinTone {
    match tone {
        SkinTone::Default => emojis::SkinTone::Default,
        SkinTone::Light => emojis::SkinTone::Light,
        SkinTone::MediumLight => emojis::SkinTone::MediumLight,
        SkinTone::Medium => emojis::SkinTone::Medium,
        SkinTone::MediumDark => emojis::SkinTone::MediumDark,
        SkinTone::Dark => emojis::SkinTone::Dark,
    }
}

#[cfg(test)]
mod tests {
    use sill_launcher::matching::matcher;
    use sill_launcher::{Frecency, ItemId, ItemKey, Needle, ProviderKind, SkinTone, UnixTime};

    use super::{emoji_items, toned};
    use crate::providers::emoji::table::emoji_table;

    const NOW: UnixTime = UnixTime(1_790_000_000);

    fn glyphs(text: &str, frecency: &Frecency) -> Vec<String> {
        emoji_items(
            emoji_table(),
            &Needle::parse(text),
            SkinTone::Default,
            frecency,
            NOW,
            5,
            &mut matcher(),
        )
        .into_iter()
        .map(|item| item.title)
        .collect()
    }

    #[test]
    fn names_keywords_and_shortcodes_all_find_an_emoji() {
        assert_eq!(
            glyphs("thumbs up", &Frecency::default())
                .first()
                .map(String::as_str),
            Some("👍")
        );
        // The M9 freeze's own verify wording (FINDINGS "M9 lane c"): a partial name match.
        assert_eq!(
            glyphs("thumbs", &Frecency::default())
                .first()
                .map(String::as_str),
            Some("👍")
        );
        assert!(
            glyphs("grin", &Frecency::default()).contains(&"😀".to_owned()),
            "a keyword"
        );
        assert!(
            glyphs(":tada", &Frecency::default()).contains(&"🎉".to_owned()),
            "a shortcode"
        );
    }

    /// Q350: "lantern" lists the lantern and the jack-o-lantern, and no flag whose name merely
    /// holds its letters in order ("flag: Netherlands").
    #[test]
    fn scattered_letters_do_not_match() {
        let lantern = glyphs("lantern", &Frecency::default());
        assert_eq!(lantern.first().map(String::as_str), Some("🏮"));
        assert!(lantern.contains(&"🎃".to_owned()), "jack-o-lantern");
        for flag in ["🇲🇵", "🇳🇱", "🇧🇶", "🇹🇫"] {
            assert!(!lantern.contains(&flag.to_owned()), "{flag} in {lantern:?}");
        }
        assert_eq!(lantern, ["🏮", "🎃"], "nothing else begins a word with it");
        assert!(
            glyphs("neth", &Frecency::default()).contains(&"🇳🇱".to_owned()),
            "the country's own word still finds its flag"
        );
    }

    #[test]
    fn recent_emoji_lead_the_empty_grid() {
        let frecency = Frecency::default().record(
            ItemKey::of(ProviderKind::Emoji, &ItemId("🎉".to_owned())),
            NOW,
        );
        assert_eq!(
            glyphs("", &frecency).first().map(String::as_str),
            Some("🎉")
        );
    }

    #[test]
    fn a_tone_applies_only_where_it_can() {
        let cases: &[(&str, SkinTone, &str)] = &[
            ("👋", SkinTone::Default, "👋"),
            ("👋", SkinTone::Medium, "👋🏽"),
            ("😀", SkinTone::Dark, "😀"),
            ("🧑‍💻", SkinTone::Light, "🧑🏻‍💻"),
        ];
        for &(glyph, tone, want) in cases {
            assert_eq!(toned(glyph, tone), want, "{glyph} {tone:?}");
        }
    }
}
