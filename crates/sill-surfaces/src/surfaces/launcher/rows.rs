//! The M9 row kinds (FINDINGS "M9 freeze" F656): which shape a row takes in the palette, and
//! the words its detail line and trail carry. Pure; lane d fills [`row_detail`].
//!
//! A file row is the file's icon (or a thumbnail), its name, its folder as `~/…` and when it
//! was modified; a clipboard row an excerpt (or an image's thumbnail) with its age; an emoji a
//! cell of the grid ([`super::emoji_grid::EmojiGrid`]); the web fallback a plain row with the
//! engine's host. quire draws each (Q290, Q291): until its `ListRow` variants or palette row
//! slots exist, these rows go through the palette's `MenuEntry::Item` as title, detail and a
//! `Trail::Note`, which says the same with less shape.

use sill_launcher::{Item, Subject, UnixTime};

/// The shape a row takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowKind {
    /// An M4 row: tile, title, detail.
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
    let _ = (item, now);
    todo!("M9 fill: d-preview")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sill_launcher::{
        FileFacts, IconRef, Item, ItemId, Mime, ProviderKind, Score, Subject, UnixTime,
    };

    use super::{RowDetail, RowKind, row_detail, row_kind};

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
    #[ignore = "M9 fill: d-preview"]
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
