//! Matching emoji. Pure; lane c fills it.
//!
//! Typed text matches the short name first (a whole-word hit on the name ranks above a
//! keyword), then keywords, then shortcodes (`:smile` with a leading colon searches shortcodes
//! and names only); nothing typed with the Emoji filter lists the recent ones (the runner's
//! frecency, `emoji:<glyph>` keys) then the rest in Unicode order: Spotlight's recent-first
//! grid. The skin tone applies to every emoji that takes one; the frecency key is always the
//! default-tone glyph, so changing the tone keeps the recents.

use nucleo_matcher::Matcher;
use sill_launcher::{Frecency, Item, Needle, SkinTone, UnixTime};

use super::table::EmojiEntry;

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
    let _ = (table, needle, tone, frecency, now, limit, matcher);
    todo!("M9 fill: c-emoji-web-units")
}

/// `glyph` with `tone` applied (Unicode's modifier after the base, inside a ZWJ sequence where
/// Unicode puts it), or as it is when it takes none or `tone` is the default.
pub fn toned(glyph: &str, tone: SkinTone) -> String {
    let _ = (glyph, tone);
    todo!("M9 fill: c-emoji-web-units")
}

#[cfg(test)]
mod tests {
    use sill_launcher::{Frecency, ItemId, ItemKey, Needle, ProviderKind, SkinTone, UnixTime};
    use sill_launcher::matching::matcher;

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
    #[ignore = "M9 fill: c-emoji-web-units"]
    fn names_keywords_and_shortcodes_all_find_an_emoji() {
        assert_eq!(glyphs("thumbs up", &Frecency::default()).first().map(String::as_str), Some("👍"));
        assert!(glyphs("grin", &Frecency::default()).contains(&"😀".to_owned()), "a keyword");
        assert!(glyphs(":tada", &Frecency::default()).contains(&"🎉".to_owned()), "a shortcode");
    }

    #[test]
    #[ignore = "M9 fill: c-emoji-web-units"]
    fn recent_emoji_lead_the_empty_grid() {
        let frecency = Frecency::default()
            .record(ItemKey::of(ProviderKind::Emoji, &ItemId("🎉".to_owned())), NOW);
        assert_eq!(glyphs("", &frecency).first().map(String::as_str), Some("🎉"));
    }

    #[test]
    #[ignore = "M9 fill: c-emoji-web-units"]
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
