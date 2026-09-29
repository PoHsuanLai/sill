//! The published sections as the palette's groups: `sill_launcher::shown` decides which rows each section shows; this names them for
//! quire's `CommandPalette` and says what each header's trailing action is ("Show More" or
//! "Show Less", quire's `PaletteGroup::with_action`).
//!
//! [`palette_groups`] is what the panel draws: each section a `PaletteGroup` with its header's
//! action, the Emoji section quire's emoji grid, a file row quire's `RowShape::File` and a
//! clipboard row `RowShape::Clip`.

use dioxus::prelude::*;
use ds::{
    ClipBody, EMOJI_CELL, EMOJI_COLUMNS, EmojiCell, EmojiCells, ImageSize, ImageSource, MenuEntry,
    MenuRow, PaletteGroup, PaletteGroups, RowShape, Tile, Trail,
};
use sill_launcher::{ClipKey, ClipKind, Item, More, Section, ShownSection, Subject, UnixTime};

use crate::surfaces::launcher::entries::{RowRef, kind_trail, row_chord};
use crate::surfaces::launcher::groups::Mix;
use crate::surfaces::launcher::rows::row_detail;

/// A section header's trailing action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeaderAction {
    None,
    ShowMore,
    ShowLess,
}

impl HeaderAction {
    /// The action for a section's `more`.
    pub fn of(more: More) -> HeaderAction {
        match more {
            More::Nothing => HeaderAction::None,
            More::Hidden(_) => HeaderAction::ShowMore,
            More::Expanded => HeaderAction::ShowLess,
        }
    }

    /// Its words.
    pub fn label(self) -> Option<&'static str> {
        match self {
            HeaderAction::None => None,
            HeaderAction::ShowMore => Some("Show More"),
            HeaderAction::ShowLess => Some("Show Less"),
        }
    }
}

/// What the rows are drawn with: each row's tile, a clipboard image's pixels and the clock.
/// The selection's chord is quire's to place (`RowChord::on_selected`, F1040).
pub struct Draw<'a> {
    pub tile_of: &'a dyn Fn(&Item) -> Tile,
    pub clip_image: &'a dyn Fn(ClipKey) -> Option<ImageSource>,
    pub now: UnixTime,
}

impl std::fmt::Debug for Draw<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Draw")
            .field("now", &self.now)
            .finish_non_exhaustive()
    }
}

/// `shown` as quire's palette groups: rows (file and clipboard rows in their shapes) or, for
/// Emoji, the grid; a capped or expanded section's header offering "Show More" or "Show Less",
/// which runs `on_more(section)`.
pub fn palette_groups(
    shown: &[ShownSection],
    draw: &Draw<'_>,
    on_more: &dyn Fn(Section) -> EventHandler<()>,
) -> PaletteGroups<RowRef> {
    let groups = shown
        .iter()
        .map(|section| {
            let title = section.section.title();
            let group = match section.section {
                Section::Emoji => PaletteGroup::grid(title, emoji_cells(&section.rows)),
                _ => PaletteGroup::list(
                    title,
                    section
                        .rows
                        .iter()
                        .map(|item| draw.shaped(item, mix_of(section.section)))
                        .collect(),
                ),
            };
            match HeaderAction::of(section.more).label() {
                Some(label) => group.with_action(label, on_more(section.section)),
                None => group,
            }
        })
        .collect::<Vec<_>>();
    PaletteGroups(groups)
}

/// The Emoji section's rows as grid cells: the glyph and its name.
fn emoji_cells(rows: &[Item]) -> EmojiCells<RowRef> {
    let cells = rows
        .iter()
        .map(|item| {
            let (glyph, name) = match &item.subject {
                Subject::Emoji(emoji) => (emoji.glyph.clone(), emoji.name.clone()),
                _ => (item.title.clone(), item.subtitle.clone()),
            };
            EmojiCell {
                value: RowRef::of(item),
                glyph,
                name,
            }
        })
        .collect();
    EmojiCells {
        cells,
        columns: EMOJI_COLUMNS,
        cell: EMOJI_CELL,
    }
}

impl Draw<'_> {
    /// `item` as a plain row: title, detail, the trail (a file's date, a clip's age, else a
    /// mixed section's kind) and its chord, which quire shows while it is the selection.
    fn plain(&self, item: &Item, mix: Mix) -> MenuEntry<RowRef> {
        let words = row_detail(item, self.now);
        let trail = match words.trail.is_empty() {
            false => Trail::Note(words.trail.clone()),
            true => kind_trail(item, mix),
        };
        MenuEntry::Row(MenuRow {
            detail: (!words.detail.is_empty()).then(|| words.detail.into()),
            tile: Some((self.tile_of)(item)),
            trail,
            chord: row_chord(item),
            ..MenuRow::new(RowRef::of(item), item.title.clone())
        })
    }

    /// `item` in its shape: a file's folder and date, a clipboard entry's text or picture and
    /// age (both in the shape, so the trail keeps only a mixed section's kind), else
    /// [`Draw::plain`].
    fn shaped(&self, item: &Item, mix: Mix) -> MenuEntry<RowRef> {
        let words = row_detail(item, self.now);
        let shape = match &item.subject {
            Subject::File(file) => RowShape::File {
                thumb: thumb_of(&file.mime.0, &file.path),
                location: words.detail.clone(),
                modified: words.trail.clone(),
            },
            Subject::Clip(clip) => RowShape::Clip {
                body: self.clip_body(clip.key, &clip.kind),
                age: words.trail.clone(),
            },
            _ => return self.plain(item, mix),
        };
        MenuEntry::Row(MenuRow {
            tile: Some((self.tile_of)(item)),
            trail: kind_trail(item, mix),
            chord: row_chord(item),
            shape,
            ..MenuRow::new(RowRef::of(item), item.title.clone())
        })
    }

    fn clip_body(&self, key: ClipKey, kind: &ClipKind) -> ClipBody {
        match kind {
            ClipKind::Text { excerpt, .. } => ClipBody::Text {
                excerpt: excerpt.0.clone(),
                lines: CLIP_LINES,
            },
            ClipKind::Image { width, height, .. } => match (self.clip_image)(key) {
                Some(src) => ClipBody::Image {
                    src,
                    size: ImageSize {
                        width: *width,
                        height: *height,
                    },
                },
                None => ClipBody::Text {
                    excerpt: format!("Image, {width} × {height}"),
                    lines: 1,
                },
            },
        }
    }
}

/// A clipboard text's lines in a row.
const CLIP_LINES: u8 = 2;

/// A picture file's own pixels as its row's thumbnail (the renderer decodes these).
fn thumb_of(mime: &str, path: &std::path::Path) -> Option<ImageSource> {
    const PICTURES: [&str; 4] = ["image/png", "image/jpeg", "image/webp", "image/gif"];
    PICTURES
        .contains(&mime)
        .then(|| ImageSource::file(path).ok())
        .flatten()
}

/// Whether a section mixes providers, so a row's kind is worth a note.
fn mix_of(section: Section) -> Mix {
    match section {
        Section::TopHit | Section::Recent => Mix::Mixed,
        _ => Mix::Single,
    }
}

#[cfg(test)]
#[path = "sections_view_tests.rs"]
mod sections_view_tests;
