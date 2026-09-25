//! The body as a card draws it: its runs' text (quire's `Text` runs carry no italic, underline or
//! link yet, FINDINGS Q124), clipped to what two lines hold collapsed and six expanded. Blitz has
//! no `line-clamp` (quire CONSUMING.md §8), so the lines are counted in characters with
//! `ds::clip_chars`, which ends a cut line with an ellipsis; the stylesheet's `max-height` in
//! `em` is the hard stop behind it. A quire card with its own line clamp (Q120) replaces both.

use sill_services::Body;

use super::Expansion;

/// About how many characters of ui 13 fit one body line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineChars(pub usize);

impl LineChars {
    /// A card `width` px wide with `padding` each side and an `icon` and a `gap` beside the
    /// text: its column over an average ui 13 glyph of 6.5 px, never under eight.
    pub fn of(width: u16, padding: u16, icon: u16, gap: u16) -> LineChars {
        let taken = 2 * u32::from(padding) + u32::from(icon) + u32::from(gap);
        let column = u32::from(width).saturating_sub(taken);
        LineChars(usize::try_from(column * 2 / 13).unwrap_or(0).max(8))
    }
}

/// How many lines the body shows (design/13 §13.3.6: two, six when expanded).
pub fn lines(expansion: Expansion) -> usize {
    match expansion {
        Expansion::Collapsed => 2,
        Expansion::Expanded => 6,
    }
}

/// `body`'s text as drawn: one paragraph collapsed (line breaks become spaces), clipped to the
/// lines `expansion` shows.
pub fn body_text(body: &Body, expansion: Expansion, per_line: LineChars) -> String {
    let text = body.text();
    let text = match expansion {
        Expansion::Collapsed => text.split_whitespace().collect::<Vec<_>>().join(" "),
        Expansion::Expanded => text.trim().to_owned(),
    };
    ds::clip_chars(&text, per_line.0 * lines(expansion))
}

#[cfg(test)]
mod tests {
    use super::{LineChars, body_text};
    use crate::surfaces::notification_card::Expansion;
    use sill_services::Body;

    #[test]
    fn the_body_is_clipped_to_its_lines() {
        let per = LineChars(10);
        let cases = [
            ("short", Expansion::Collapsed, "short"),
            ("two\nlines", Expansion::Collapsed, "two lines"),
            ("two\nlines", Expansion::Expanded, "two\nlines"),
        ];
        for (raw, expansion, want) in cases {
            assert_eq!(
                body_text(&Body::plain(raw), expansion, per),
                want,
                "{raw:?}"
            );
        }
        let long = Body::plain(&"a".repeat(80));
        let collapsed = body_text(&long, Expansion::Collapsed, per);
        assert_eq!(collapsed.chars().count(), 20);
        assert!(collapsed.ends_with('…'), "{collapsed}");
        let expanded = body_text(&long, Expansion::Expanded, per);
        assert_eq!(expanded.chars().count(), 60);
    }

    #[test]
    fn a_banner_line_holds_about_forty_five_characters() {
        assert_eq!(LineChars::of(360, 12, 32, 10), LineChars(45));
        assert_eq!(
            LineChars::of(100, 40, 32, 10),
            LineChars(8),
            "never under eight"
        );
    }
}
