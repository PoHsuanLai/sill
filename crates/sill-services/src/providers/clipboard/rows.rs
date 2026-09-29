//! A history entry as a row, at a given time (so the age words are testable). Pure. A row
//! carries an excerpt of a text, never the whole of it.

use nucleo_matcher::Matcher;
use sill_launcher::{
    Action, ActionId, ActionKeys, CharCount, ClipFacts, ClipKind, Excerpt, ExcerptLimit,
    FrecencyScore, IconRef, Item, ItemId, Needle, Prefix, ProviderKind, RankWeights, Subject,
    UnixTime, chords, rank,
};

use super::FORGET_ACTION;
use sill_model::clipboard::{ClipContent, ClipEntry, ClipImage, Clipboard};

/// A title's most characters.
const TITLE_CHARS: usize = 80;

/// How much of a text is matched against (a long copy matches on its beginning).
const MATCH_CHARS: usize = 10_000;

/// [`super::clip_items`] at `now`.
pub(super) fn rows_at(
    history: &Clipboard,
    needle: &Needle,
    matcher: &mut Matcher,
    now: UnixTime,
) -> Vec<Item> {
    let mut rows: Vec<Item> = history
        .entries
        .iter()
        .filter_map(|entry| row(entry, needle, matcher, now))
        .collect();
    // Stable: equals keep the history's newest-first order.
    rows.sort_by_key(|row| std::cmp::Reverse(row.score));
    rows
}

fn row(entry: &ClipEntry, needle: &Needle, matcher: &mut Matcher, now: UnixTime) -> Option<Item> {
    let (title, haystack, detail, kind) = match &entry.content {
        ClipContent::Text(text) => {
            let chars = text.0.chars().count();
            (
                title_of(&text.0),
                text.0.chars().take(MATCH_CHARS).collect::<String>(),
                characters(chars),
                ClipKind::Text {
                    excerpt: excerpt(&text.0, ExcerptLimit::PANE),
                    chars: CharCount(u32::try_from(chars).unwrap_or(u32::MAX)),
                },
            )
        }
        ClipContent::Image(image) => (
            "Image".to_owned(),
            format!("Image {} {}", short_type(image), image.mime.0),
            format!(
                "{}\u{d7}{} {}",
                image.width,
                image.height,
                short_type(image)
            ),
            ClipKind::Image {
                mime: image.mime.clone(),
                width: image.width,
                height: image.height,
            },
        ),
    };
    let matched = needle.score(&haystack, matcher)?;
    let prefix = Prefix::of(needle.text(), &title);
    Some(Item {
        id: ItemId(entry.key.0.to_string()),
        provider: ProviderKind::Clipboard,
        subtitle: format!("{} \u{b7} {detail}", age(now, entry.copied)),
        title,
        icon: IconRef::None,
        score: rank(matched, FrecencyScore::NONE, prefix, &RankWeights::DEFAULT),
        actions: vec![Action {
            id: ActionId(FORGET_ACTION.to_owned()),
            label: "Forget".to_owned(),
            keys: ActionKeys::Chord(chords::forget()),
        }],
        subject: Subject::Clip(ClipFacts {
            key: entry.key,
            kind,
            copied: entry.copied,
        }),
    })
}

/// The first line that is not blank, trimmed, at most [`TITLE_CHARS`] characters (an
/// ellipsis marks a cut).
fn title_of(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    if line.chars().count() <= TITLE_CHARS {
        return line.to_owned();
    }
    let cut: String = line.chars().take(TITLE_CHARS - 1).collect();
    format!("{}\u{2026}", cut.trim_end())
}

/// The first lines of `text` within `limit`, cut on a character, trailing whitespace trimmed.
fn excerpt(text: &str, limit: ExcerptLimit) -> Excerpt {
    let lines = usize::try_from(limit.lines).unwrap_or(usize::MAX);
    let chars = usize::try_from(limit.chars).unwrap_or(usize::MAX);
    let head: String = text
        .split_inclusive('\n')
        .take(lines)
        .collect::<String>()
        .chars()
        .take(chars)
        .collect();
    Excerpt(head.trim_end().to_owned())
}

/// `PNG` for `image/png`.
fn short_type(image: &ClipImage) -> String {
    image
        .mime
        .0
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase()
}

/// "1 character", "1,204 characters".
fn characters(count: usize) -> String {
    match count {
        1 => "1 character".to_owned(),
        n => format!("{} characters", grouped(n)),
    }
}

/// `n` with commas between thousands.
fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let first = digits.len() % 3;
    digits
        .char_indices()
        .flat_map(|(i, digit)| {
            let comma = i > 0 && (i + 3 - first).is_multiple_of(3);
            comma.then_some(',').into_iter().chain(Some(digit))
        })
        .collect()
}

/// How long ago `copied` was, in words.
fn age(now: UnixTime, copied: UnixTime) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    match now.0.saturating_sub(copied.0) {
        s if s < MINUTE => "Just now".to_owned(),
        s if s < HOUR => format!("{} min ago", s / MINUTE),
        s if s < DAY => format!("{} h ago", s / HOUR),
        s if s < 2 * DAY => "Yesterday".to_owned(),
        s => format!("{} days ago", s / DAY),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sill_launcher::{ClipKey, ExcerptLimit, Mime, Needle, UnixTime, matching::matcher};

    use super::{age, excerpt, grouped, rows_at, title_of};
    use sill_model::clipboard::{ClipContent, ClipEntry, ClipImage, ClipText, Clipboard};

    #[test]
    fn a_title_is_the_first_line_that_says_something() {
        let long = "x".repeat(100);
        let cases = [
            ("made-up line\nsecond", "made-up line".to_owned()),
            ("\n\n   indented words  \nmore", "indented words".to_owned()),
            (long.as_str(), format!("{}\u{2026}", "x".repeat(79))),
            ("", String::new()),
        ];
        for (text, want) in cases {
            assert_eq!(title_of(text), want);
        }
    }

    #[test]
    fn an_excerpt_keeps_its_first_lines_within_the_limit() {
        let limit = ExcerptLimit { lines: 2, chars: 8 };
        let cases = [
            ("a\nb\nc", "a\nb"),
            ("abcdefghijk", "abcdefgh"),
            (
                "\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}",
                "\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}\u{e9}",
            ),
            ("ab  \n", "ab"),
        ];
        for (text, want) in cases {
            assert_eq!(excerpt(text, limit).0, want);
        }
    }

    #[test]
    fn ages_and_counts_read_as_words() {
        let cases = [
            (100, 100, "Just now"),
            (100, 200, "Just now"),
            (3_700, 100, "1 h ago"),
            (220, 100, "2 min ago"),
            (100 + 86_400 + 5, 100, "Yesterday"),
            (100 + 3 * 86_400, 100, "3 days ago"),
        ];
        for (now, copied, want) in cases {
            assert_eq!(age(UnixTime(now), UnixTime(copied)), want, "{now} {copied}");
        }
        let counts = [
            (0, "0"),
            (999, "999"),
            (1_204, "1,204"),
            (1_234_567, "1,234,567"),
        ];
        for (n, want) in counts {
            assert_eq!(grouped(n), want);
        }
    }

    #[test]
    fn rows_say_their_age_and_size_and_images_match_their_type() {
        let history = Clipboard {
            entries: vec![
                ClipEntry {
                    key: ClipKey(2),
                    content: ClipContent::Image(ClipImage {
                        mime: Mime("image/png".to_owned()),
                        bytes: Arc::from(vec![0_u8; 4]),
                        width: 640,
                        height: 480,
                    }),
                    copied: UnixTime(1_000),
                },
                ClipEntry {
                    key: ClipKey(1),
                    content: ClipContent::Text(ClipText(Arc::from("made-up\nwords"))),
                    copied: UnixTime(880),
                },
            ],
            ..Clipboard::default()
        };
        let at = UnixTime(1_000);
        let shown = |text: &str| -> Vec<(String, String)> {
            rows_at(&history, &Needle::parse(text), &mut matcher(), at)
                .into_iter()
                .map(|row| (row.title, row.subtitle))
                .collect()
        };
        let image = (
            "Image".to_owned(),
            "Just now \u{b7} 640\u{d7}480 PNG".to_owned(),
        );
        let text = (
            "made-up".to_owned(),
            "2 min ago \u{b7} 13 characters".to_owned(),
        );
        assert_eq!(shown("png"), std::slice::from_ref(&image));
        assert_eq!(shown("words"), std::slice::from_ref(&text));
        assert_eq!(
            shown(""),
            [image, text],
            "newest first when nothing is typed"
        );
    }

    #[test]
    fn a_prefix_of_the_title_ranks_above_a_match_further_in() {
        let entry = |key, text: &str| ClipEntry {
            key: ClipKey(key),
            content: ClipContent::Text(ClipText(Arc::from(text))),
            copied: UnixTime(0),
        };
        let history = Clipboard {
            entries: vec![entry(2, "the made-up plan"), entry(1, "plan for later")],
            ..Clipboard::default()
        };
        let rows = rows_at(
            &history,
            &Needle::parse("plan"),
            &mut matcher(),
            UnixTime(0),
        );
        let ids: Vec<&str> = rows.iter().map(|r| r.id.0.as_str()).collect();
        assert_eq!(ids, ["1", "2"]);
        assert!(rows[0].score > rows[1].score);
    }
}
