//! The published sections as the palette's groups (M9 lane d, FINDINGS "M9 freeze" F645,
//! F656): `sill_launcher::shown` decides which rows each section shows; this names them for
//! quire's `CommandPalette` and says what each header's trailing action is ("Show More" or
//! "Show Less", quire's `SectionHeader` action: Q294).

use ds::{MenuEntry, Tile};
use sill_launcher::{Item, More, ShownSection};

use crate::surfaces::launcher::entries::RowRef;

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
/// row trailing `↵` and rows with actions their first chord or `⌃K` (the M4 rule, `entries.rs`),
/// file and clipboard rows their `rows::row_detail` trail. The emoji section is left out when
/// the grid draws it. Lane d fills it.
pub fn palette_sections(
    shown: &[ShownSection],
    tile_of: &dyn Fn(&Item) -> Tile,
) -> Vec<(String, Vec<MenuEntry<RowRef>>)> {
    let _ = (shown, tile_of);
    todo!("M9 fill: d-preview")
}

#[cfg(test)]
mod tests {
    use sill_launcher::More;

    use super::HeaderAction;

    #[test]
    fn a_capped_section_offers_more_and_an_expanded_one_less() {
        assert_eq!(HeaderAction::of(More::Hidden(3)).label(), Some("Show More"));
        assert_eq!(HeaderAction::of(More::Expanded).label(), Some("Show Less"));
        assert_eq!(HeaderAction::of(More::Nothing).label(), None);
    }
}
