//! The published sections as the palette's groups (M9 lane d, FINDINGS "M9 freeze" F645,
//! F656): `sill_launcher::shown` decides which rows each section shows; this names them for
//! quire's `CommandPalette` and says what each header's trailing action is ("Show More" or
//! "Show Less", quire's `PaletteGroup::with_action`, Q294).
//!
//! [`palette_groups`] is what the panel draws: each section a `PaletteGroup` with its header's
//! action, the Emoji section quire's emoji grid (Q291), a file row quire's `RowShape::File` and a
//! clipboard row `RowShape::Clip` (Q290). [`palette_sections`] (the freeze's) is the same rows
//! as plain lists, for a picture of the results without handlers.

use dioxus::prelude::*;
use ds::{
    Availability, ClipBody, EMOJI_CELL, EMOJI_COLUMNS, EmojiCell, EmojiCells, ImageSize,
    ImageSource, MenuEntry, MenuRow, PaletteGroup, PaletteGroups, RowShape, Shortcut, Tile, Trail,
};
use sill_launcher::{ClipKey, ClipKind, Item, More, Section, ShownSection, Subject, UnixTime};

use crate::surfaces::launcher::chord_keys::action_shortcut;
use crate::surfaces::launcher::entries::{Cursor, Enter, RowRef, kind_note};
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

/// `shown` as the palette's groups (title, rows), each row's tile from `tile_of`, the first
/// row (where a palette's cursor starts) trailing `↵` and rows with actions their first chord
/// or `⌃K` (the M4 rule, `entries.rs`), file and clipboard rows their `rows::row_detail` trail.
/// No header actions and no grid: the panel draws [`palette_groups`].
pub fn palette_sections(
    shown: &[ShownSection],
    tile_of: &dyn Fn(&Item) -> Tile,
) -> Vec<(String, Vec<MenuEntry<RowRef>>)> {
    let draw = Draw {
        tile_of,
        clip_image: &|_| None,
        now: now(),
        cursor: Cursor::default(),
    };
    let mut lines = 0..;
    shown
        .iter()
        .map(|section| {
            let entries = section
                .rows
                .iter()
                .zip(&mut lines)
                .map(|(item, line)| draw.plain(item, mix_of(section.section), line))
                .collect();
            (section.section.title().to_owned(), entries)
        })
        .collect()
}

/// What the rows are drawn with: each row's tile, a clipboard image's pixels, the clock, and
/// the palette's cursor (the row there trails `↵`: Enter runs it, F742).
pub struct Draw<'a> {
    pub tile_of: &'a dyn Fn(&Item) -> Tile,
    pub clip_image: &'a dyn Fn(ClipKey) -> Option<ImageSource>,
    pub now: UnixTime,
    pub cursor: Cursor,
}

impl std::fmt::Debug for Draw<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Draw")
            .field("now", &self.now)
            .field("cursor", &self.cursor)
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
    let mut start = 0;
    let groups = shown
        .iter()
        .map(|section| {
            let title = section.section.title();
            let first = start;
            start += section_stops(section);
            let group = match section.section {
                Section::Emoji => PaletteGroup::grid(title, emoji_cells(&section.rows)),
                _ => PaletteGroup::list(
                    title,
                    section
                        .rows
                        .iter()
                        .zip(first..)
                        .map(|(item, line)| draw.shaped(item, mix_of(section.section), line))
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

/// The cursor's stops in `section`, as the palette counts them: each row (each emoji cell),
/// then the header's action when it has one (`view::Line`).
fn section_stops(section: &ShownSection) -> usize {
    let action = match HeaderAction::of(section.more) {
        HeaderAction::None => 0,
        HeaderAction::ShowMore | HeaderAction::ShowLess => 1,
    };
    section.rows.len() + action
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
    /// `item` on `line` as a plain row: title, detail, the M4 trail (a file's date, a clip's
    /// age), `↵` while the cursor is on it.
    fn plain(&self, item: &Item, mix: Mix, line: usize) -> MenuEntry<RowRef> {
        let words = row_detail(item, self.now);
        let enter = self.cursor.on(line);
        let trail = match (words.trail.is_empty(), enter) {
            (false, Enter::Not) => Trail::Note(words.trail.clone()),
            _ => trail(item, mix, enter),
        };
        MenuEntry::Item {
            value: RowRef::of(item),
            title: item.title.clone(),
            detail: (!words.detail.is_empty()).then_some(words.detail),
            tile: Some((self.tile_of)(item)),
            trail,
            check: None,
            availability: Availability::Enabled,
        }
    }

    /// `item` in its shape: a file's folder and date, a clipboard entry's text or picture and
    /// age, else [`Draw::plain`]; on `line`.
    fn shaped(&self, item: &Item, mix: Mix, line: usize) -> MenuEntry<RowRef> {
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
            _ => return self.plain(item, mix, line),
        };
        MenuEntry::Row(MenuRow {
            tile: Some((self.tile_of)(item)),
            trail: trail(item, mix, self.cursor.on(line)),
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

/// `↵` on the row Enter runs; else the first chord of its actions, `⌃K` when none has one;
/// else, in a mixed section, its kind.
fn trail(item: &Item, mix: Mix, enter: Enter) -> Trail {
    let chord = item
        .actions
        .iter()
        .find_map(|action| action_shortcut(&action.keys));
    match (enter, chord, item.actions.is_empty(), mix) {
        (Enter::Runs, ..) => Trail::Shortcut(Shortcut(vec![ds::Key::Enter])),
        (Enter::Not, Some(chord), ..) => Trail::Shortcut(chord),
        (Enter::Not, None, false, _) => {
            Trail::Shortcut(Shortcut(vec![ds::Key::Ctrl, ds::Key::Char('k')]))
        }
        (Enter::Not, None, true, Mix::Mixed) => Trail::Note(kind_note(item.provider).to_owned()),
        (Enter::Not, None, true, Mix::Single) => Trail::None,
    }
}

fn now() -> UnixTime {
    UnixTime(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs()),
    )
}

#[cfg(test)]
#[path = "sections_view_tests.rs"]
mod sections_view_tests;
