//! The row kinds: which shape a row takes in the palette, and the words its detail line and
//! trail carry. Pure but for the local time zone [`row_detail`] reads.
//!
//! A file row is the file's icon (or a thumbnail), its name, its folder as `~/…` and when it
//! was modified; a clipboard row an excerpt (or an image's thumbnail) with its age; an emoji a
//! cell of the grid ([`super::emoji_grid::EmojiGrid`]); the web fallback a plain row with the
//! engine's host. quire draws each: `sections_view::palette_groups` hands these
//! words to `RowShape::File` and `RowShape::Clip`, and the emoji to the palette's grid.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use sill_launcher::{Item, Subject, UnixTime};
use sill_services::UnixMs;

use crate::surfaces::notification_card::labels::{age, age_label};

/// The shape a row takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowKind {
    /// A plain row: tile, title, detail.
    Plain,
    File,
    Clip,
    /// A grid cell, not a row.
    EmojiCell,
    Web,
}

/// `item`'s kind, from its subject.
pub fn row_kind(item: &Item) -> RowKind {
    match item.subject {
        Subject::None | Subject::App(_) => RowKind::Plain,
        Subject::File(_) => RowKind::File,
        Subject::Clip(_) => RowKind::Clip,
        Subject::Emoji(_) => RowKind::EmojiCell,
        Subject::Web(_) => RowKind::Web,
    }
}

/// A row's words beyond its title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowDetail {
    /// The faint line under the title.
    pub detail: String,
    /// The trailing data (a date, an age, a size), or empty.
    pub trail: String,
}

/// `item`'s detail at `now`. File: detail its folder (the provider's subtitle), trail its
/// modification as the mail list words a time ("14:05" today, "Yesterday", "Mon", "12 Sep",
/// "12 Sep 2025"); clipboard: detail the provider's subtitle, trail its age ("now", "5m",
/// "2h", "3d", the notification card's `age_label`); web: detail the engine; others: the
/// subtitle and no trail.
pub fn row_detail(item: &Item, now: UnixTime) -> RowDetail {
    detail_in(item, now, &TimeZone::system())
}

/// [`row_detail`] in `zone` (tests pin one).
fn detail_in(item: &Item, now: UnixTime, zone: &TimeZone) -> RowDetail {
    let (detail, trail) = match &item.subject {
        Subject::File(facts) => (item.subtitle.clone(), when(facts.modified, now, zone)),
        Subject::Clip(facts) => (
            item.subtitle.clone(),
            age_label(age(ms(facts.copied), ms(now))),
        ),
        Subject::Web(facts) => (facts.engine.clone(), String::new()),
        Subject::None | Subject::App(_) | Subject::Emoji(_) => {
            (item.subtitle.clone(), String::new())
        }
    };
    RowDetail { detail, trail }
}

fn ms(at: UnixTime) -> UnixMs {
    UnixMs(i64::try_from(at.0.saturating_mul(1000)).unwrap_or(i64::MAX))
}

/// `at` as the mail list words a time at `now`: `14:05` today, `Yesterday`, a weekday within
/// the week, `12 Sep` within the year, else `12 Sep 2025`. A time in the future reads as a date.
fn when(at: UnixTime, now: UnixTime, zone: &TimeZone) -> String {
    let zoned = |t: UnixTime| {
        i64::try_from(t.0)
            .ok()
            .and_then(|secs| Timestamp::from_second(secs).ok())
            .map(|stamp| stamp.to_zoned(zone.clone()))
    };
    let (Some(at), Some(now)) = (zoned(at), zoned(now)) else {
        return String::new();
    };
    let days = (now.date() - at.date()).get_days();
    match days {
        0 => at.strftime("%H:%M").to_string(),
        1 => "Yesterday".to_owned(),
        2..=6 => at.strftime("%a").to_string(),
        _ if at.year() == now.year() => at.strftime("%-d %b").to_string(),
        _ => at.strftime("%-d %b %Y").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sill_launcher::{
        CharCount, ClipFacts, ClipKey, ClipKind, Excerpt, FileFacts, IconRef, Item, ItemId, Mime,
        ProviderKind, Score, Subject, UnixTime, WebFacts,
    };

    use super::{RowDetail, RowKind, detail_in, row_detail, row_kind};

    const NOW: UnixTime = UnixTime(1_790_000_000);

    fn file_row(modified: UnixTime) -> Item {
        Item {
            id: ItemId("/home/p/Documents/a.pdf".to_owned()),
            provider: ProviderKind::Files,
            title: "a.pdf".to_owned(),
            subtitle: "~/Documents".to_owned(),
            icon: IconRef::None,
            score: Score(0),
            actions: Vec::new(),
            subject: Subject::File(FileFacts {
                path: PathBuf::from("/home/p/Documents/a.pdf"),
                mime: Mime("application/pdf".to_owned()),
                modified,
            }),
        }
    }

    #[test]
    fn a_rows_kind_is_its_subjects() {
        assert_eq!(row_kind(&file_row(NOW)), RowKind::File);
        let plain = Item {
            subject: Subject::None,
            ..file_row(NOW)
        };
        assert_eq!(row_kind(&plain), RowKind::Plain);
    }

    #[test]
    fn a_file_rows_time_is_worded_like_the_mail_lists() {
        // 1_790_000_000 is Monday 2026-09-21 14:13:20 UTC.
        const CASES: &[(u64, &str)] = &[
            (0, "14:13"),
            (3_600 * 14, "00:13"),
            (3_600 * 15, "Yesterday"),
            (86_400 * 2, "Sat"),
            (86_400 * 6, "Tue"),
            (86_400 * 7, "14 Sep"),
            (86_400 * 300, "25 Nov 2025"),
        ];
        let utc = jiff::tz::TimeZone::UTC;
        for &(ago, want) in CASES {
            let row = file_row(UnixTime(NOW.0 - ago));
            assert_eq!(detail_in(&row, NOW, &utc).trail, want, "{ago}s ago");
        }
        let later = file_row(UnixTime(NOW.0 + 86_400 * 2));
        assert_eq!(detail_in(&later, NOW, &utc).trail, "23 Sep", "the future");
    }

    #[test]
    fn a_clip_row_trails_its_age_and_a_web_row_its_engine() {
        let clip = Item {
            subject: Subject::Clip(ClipFacts {
                key: ClipKey(1),
                kind: ClipKind::Text {
                    excerpt: Excerpt("x".to_owned()),
                    chars: CharCount(1),
                },
                copied: UnixTime(NOW.0 - 300),
            }),
            subtitle: "Text".to_owned(),
            ..file_row(NOW)
        };
        assert_eq!(
            row_detail(&clip, NOW),
            RowDetail {
                detail: "Text".to_owned(),
                trail: "5m".to_owned()
            }
        );
        let web = Item {
            subject: Subject::Web(WebFacts {
                engine: "duckduckgo.com".to_owned(),
                url: "https://duckduckgo.com/?q=x".to_owned(),
            }),
            ..file_row(NOW)
        };
        assert_eq!(row_detail(&web, NOW).detail, "duckduckgo.com");
        assert_eq!(row_detail(&web, NOW).trail, "");
    }

    #[test]
    fn a_file_row_says_where_it_is_and_when_it_changed() {
        let detail = row_detail(&file_row(UnixTime(NOW.0 - 86_400)), NOW);
        assert_eq!(
            detail,
            RowDetail {
                detail: "~/Documents".to_owned(),
                trail: "Yesterday".to_owned(),
            }
        );
    }
}
