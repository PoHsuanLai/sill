//! The column's input and blur regions as element regions (shell-host's `RegionSpec::Elements`,
//! resolved after every layout, G60): each drawn card's painted parts by id, rounded by
//! `layer::CARD_RADIUS` (the close button by its own half), so a press in the gap between cards
//! passes through to the window below and nothing but the cards is blurred. While a card's
//! actions menu is open the input is the whole surface, since the menu hangs outside the cards.
//! (`regions::card_regions`, the freeze's measured-rects form, is kept for its callers' sake but
//! no longer drawn from.)

use shell_host::{BlurSpec, ElementId, ElementRegion, RegionSpec};
use sill_services::NotificationId;

use super::layer::CARD_RADIUS;
use crate::surfaces::notification_card::{CardPart, Expansion, MenuOpen, part_id};

/// The close button's corner: a circle (quire's 26 px `IconButton{Strip}` on a round `Surface`).
const CLOSE_RADIUS: i32 = 13;

/// One drawn card, as the regions see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardShape {
    /// The notification it draws.
    pub note: NotificationId,
    /// How many it stands for (more than one draws the two layers).
    pub count: usize,
    pub expansion: Expansion,
}

/// The surface's input and blur regions for `cards`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regions {
    pub input: RegionSpec,
    pub blur: BlurSpec,
}

/// The regions `cards` take, `menu` saying whether an actions menu is open.
pub fn element_regions(cards: &[CardShape], menu: MenuOpen) -> Regions {
    let painted: Vec<ElementRegion> = cards.iter().flat_map(painted).collect();
    let closes = cards.iter().filter_map(|c| match c.expansion {
        Expansion::Expanded => Some(region(c.note, CardPart::Close, CLOSE_RADIUS)),
        Expansion::Collapsed => None,
    });
    let input = match (menu, painted.is_empty()) {
        (MenuOpen::Yes, _) => RegionSpec::Whole,
        (MenuOpen::No, true) => RegionSpec::Empty,
        (MenuOpen::No, false) => {
            RegionSpec::Elements(painted.iter().cloned().chain(closes).collect())
        }
    };
    let blur = match painted.is_empty() {
        true => BlurSpec::None,
        false => BlurSpec::Region(RegionSpec::Elements(painted)),
    };
    Regions { input, blur }
}

/// A card's face, and its two layers when it stands for a group.
fn painted(card: &CardShape) -> Vec<ElementRegion> {
    let layers: &[CardPart] = match card.count {
        0 | 1 => &[],
        _ => &[CardPart::Near, CardPart::Far],
    };
    std::iter::once(&CardPart::Face)
        .chain(layers)
        .map(|&part| region(card.note, part, CARD_RADIUS))
        .collect()
}

fn region(note: NotificationId, part: CardPart, radius: i32) -> ElementRegion {
    ElementRegion {
        id: ElementId::new(part_id(note, part)),
        radius,
        inflate: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{CardShape, Regions, element_regions};
    use crate::surfaces::notification_card::{Expansion, MenuOpen};
    use shell_host::{BlurSpec, ElementId, ElementRegion, RegionSpec};
    use sill_services::NotificationId;

    fn el(id: &str, radius: i32) -> ElementRegion {
        ElementRegion {
            id: ElementId::new(id),
            radius,
            inflate: 0,
        }
    }

    fn shape(note: u32, count: usize, expansion: Expansion) -> CardShape {
        CardShape {
            note: NotificationId(note),
            count,
            expansion,
        }
    }

    #[test]
    fn the_regions_are_the_cards_painted_parts() {
        use Expansion::{Collapsed, Expanded};
        let cases: Vec<(&str, Vec<CardShape>, MenuOpen, Regions)> = vec![
            (
                "nothing",
                vec![],
                MenuOpen::No,
                Regions {
                    input: RegionSpec::Empty,
                    blur: BlurSpec::None,
                },
            ),
            (
                "a lone card, then a group hovered",
                vec![shape(1, 1, Collapsed), shape(5, 3, Expanded)],
                MenuOpen::No,
                Regions {
                    input: RegionSpec::Elements(vec![
                        el("note-1-face", 16),
                        el("note-5-face", 16),
                        el("note-5-near", 16),
                        el("note-5-far", 16),
                        el("note-5-close", 13),
                    ]),
                    blur: BlurSpec::Region(RegionSpec::Elements(vec![
                        el("note-1-face", 16),
                        el("note-5-face", 16),
                        el("note-5-near", 16),
                        el("note-5-far", 16),
                    ])),
                },
            ),
            (
                "an actions menu open",
                vec![shape(2, 1, Expanded)],
                MenuOpen::Yes,
                Regions {
                    input: RegionSpec::Whole,
                    blur: BlurSpec::Region(RegionSpec::Elements(vec![el("note-2-face", 16)])),
                },
            ),
        ];
        for (name, cards, menu, want) in cases {
            assert_eq!(element_regions(&cards, menu), want, "{name}");
        }
    }
}
