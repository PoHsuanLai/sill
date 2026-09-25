//! The ids a card writes on its painted parts, so a surface can name them in an element region
//! (shell-host's `RegionSpec::Elements`): the face, a group's two layers, and the close button.
//! Unique per document, since a notification id is.

use sill_services::NotificationId;

/// One painted part of a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardPart {
    /// The card itself.
    Face,
    /// The group layer just under it.
    Near,
    /// The group layer under that.
    Far,
    /// The close button.
    Close,
}

impl CardPart {
    fn slug(self) -> &'static str {
        match self {
            CardPart::Face => "face",
            CardPart::Near => "near",
            CardPart::Far => "far",
            CardPart::Close => "close",
        }
    }
}

/// The `id` of `part` of the card drawing notification `id`.
pub fn part_id(id: NotificationId, part: CardPart) -> String {
    format!("note-{}-{}", id.0, part.slug())
}

#[cfg(test)]
mod tests {
    use super::{CardPart, part_id};
    use sill_services::NotificationId;

    #[test]
    fn each_part_has_its_own_id() {
        let id = NotificationId(7);
        let ids = [
            CardPart::Face,
            CardPart::Near,
            CardPart::Far,
            CardPart::Close,
        ]
        .map(|p| part_id(id, p));
        assert_eq!(
            ids,
            ["note-7-face", "note-7-near", "note-7-far", "note-7-close"]
        );
    }
}
