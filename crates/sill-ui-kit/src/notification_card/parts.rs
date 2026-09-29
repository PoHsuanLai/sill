//! The ids a banner writes, so its surface can name them in an element region (shell-host's
//! `RegionSpec::Elements`): quire's plate (`NotificationCard { id }`, design/20 §1.6
//! `Element("toast-<id>")`) and sill's wrapper around the whole card, which a group's layers
//! reach below the plate. Unique per document, since a notification id is.

use sill_model::notifications::NotificationId;

/// One named part of a banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardPart {
    /// quire's plate: the card itself.
    Plate,
    /// The whole card, the group's layers under the plate included.
    Whole,
}

/// The `id` of `part` of the banner drawing notification `id`.
pub fn part_id(id: NotificationId, part: CardPart) -> String {
    match part {
        CardPart::Plate => format!("toast-{}", id.0),
        CardPart::Whole => format!("banner-{}", id.0),
    }
}

#[cfg(test)]
mod tests {
    use super::{CardPart, part_id};
    use sill_model::notifications::NotificationId;

    #[test]
    fn each_part_has_its_own_id() {
        let cases = [(CardPart::Plate, "toast-7"), (CardPart::Whole, "banner-7")];
        for (part, want) in cases {
            assert_eq!(part_id(NotificationId(7), part), want, "{part:?}");
        }
    }
}
