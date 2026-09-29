//! The column's input and blur regions as element regions (shell-host's `RegionSpec::Elements`,
//! resolved after every layout): each drawn card's quire plate by its id (`toast-<id>`,
//! design/20 §1.6), and for a group the whole card, whose layers reach below the plate, so a
//! press in the gap between cards passes through to the window below and nothing but the cards
//! is blurred. A card sliding out is still drawn, so it keeps its regions until the stack says
//! it has gone.

use shell_host::{BlurSpec, ElementId, ElementRegion, RegionSpec};
use sill_services::NotificationId;

use super::layer::CARD_RADIUS;
use crate::surfaces::notification_card::{CardPart, part_id};

/// How far quire's close button stands out past the plate's top left corner (`--s-6`): the
/// input region reaches that far round the plate so the whole button takes a press.
const CLOSE_OVERHANG: i32 = 6;

/// The input region's corner round the plate: the close button's own radius (18 / 2), so the
/// button sits exactly in the corner's arc.
const CLOSE_RADIUS: i32 = 9;

/// One drawn card, as the regions see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardShape {
    /// The notification it draws.
    pub note: NotificationId,
    /// How many it stands for (more than one draws the layers).
    pub count: usize,
}

/// The surface's input and blur regions for `cards`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regions {
    pub input: RegionSpec,
    pub blur: BlurSpec,
}

/// The regions `cards` take.
pub fn element_regions(cards: &[CardShape]) -> Regions {
    if cards.is_empty() {
        return Regions {
            input: RegionSpec::Empty,
            blur: BlurSpec::None,
        };
    }
    let input = cards
        .iter()
        .flat_map(|card| {
            let plate = region(card.note, CardPart::Plate, CLOSE_RADIUS, CLOSE_OVERHANG);
            std::iter::once(plate).chain(layers(card))
        })
        .collect();
    let blur = cards
        .iter()
        .flat_map(|card| {
            let plate = region(card.note, CardPart::Plate, CARD_RADIUS, 0);
            std::iter::once(plate).chain(layers(card))
        })
        .collect();
    Regions {
        input: RegionSpec::Elements(input),
        blur: BlurSpec::Region(RegionSpec::Elements(blur)),
    }
}

/// A group's whole card, layers and all; nothing for a lone card.
fn layers(card: &CardShape) -> Option<ElementRegion> {
    (card.count > 1).then(|| region(card.note, CardPart::Whole, CARD_RADIUS, 0))
}

fn region(note: NotificationId, part: CardPart, radius: i32, inflate: i32) -> ElementRegion {
    ElementRegion {
        id: ElementId::new(part_id(note, part)),
        radius,
        inflate,
    }
}

#[cfg(test)]
mod tests {
    use super::{CardShape, Regions, element_regions};
    use shell_host::{BlurSpec, ElementId, ElementRegion, RegionSpec};
    use sill_services::NotificationId;

    fn el(id: &str, radius: i32, inflate: i32) -> ElementRegion {
        ElementRegion {
            id: ElementId::new(id),
            radius,
            inflate,
        }
    }

    fn shape(note: u32, count: usize) -> CardShape {
        CardShape {
            note: NotificationId(note),
            count,
        }
    }

    #[test]
    fn the_regions_are_the_cards_plates_and_a_groups_layers() {
        let cases: Vec<(&str, Vec<CardShape>, Regions)> = vec![
            (
                "nothing",
                vec![],
                Regions {
                    input: RegionSpec::Empty,
                    blur: BlurSpec::None,
                },
            ),
            (
                "a lone card, then a group",
                vec![shape(1, 1), shape(5, 3)],
                Regions {
                    input: RegionSpec::Elements(vec![
                        el("toast-1", 9, 6),
                        el("toast-5", 9, 6),
                        el("banner-5", 16, 0),
                    ]),
                    blur: BlurSpec::Region(RegionSpec::Elements(vec![
                        el("toast-1", 16, 0),
                        el("toast-5", 16, 0),
                        el("banner-5", 16, 0),
                    ])),
                },
            ),
        ];
        for (name, cards, want) in cases {
            assert_eq!(element_regions(&cards), want, "{name}");
        }
    }
}
