//! The banner column's input and blur region: its cards' rounded rects, in surface-local
//! logical pixels, so clicks and blur stop at the cards (layer.rs's module doc).

use shell_host::{Rect, RegionSpec};

/// One card's box as measured after layout (surface-local, logical px), leaving ones included
/// until they have gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardBox(pub Rect);

/// The region the column takes: every card's box rounded by `layer::CARD_RADIUS`, in order;
/// empty (clicks pass through, nothing blurred) when no card shows. A box with no area is left
/// out.
pub fn card_regions(cards: &[CardBox]) -> RegionSpec {
    let _ = cards;
    todo!("M6 fill: b-banners")
}

#[cfg(test)]
mod tests {
    use super::{CardBox, card_regions};
    use crate::surfaces::banners::layer::CARD_RADIUS;
    use shell_host::{Rect, RegionSpec, RoundedRect};

    fn rect(y: i32, height: i32) -> Rect {
        Rect {
            x: 40,
            y,
            width: 360,
            height,
        }
    }

    #[test]
    #[ignore = "M6 fill: b-banners"]
    fn the_region_is_the_cards_and_nothing_else() {
        assert_eq!(card_regions(&[]), RegionSpec::Empty);
        assert_eq!(card_regions(&[CardBox(rect(40, 0))]), RegionSpec::Empty);
        let rounded = |r| RoundedRect {
            rect: r,
            radius: CARD_RADIUS,
        };
        assert_eq!(
            card_regions(&[CardBox(rect(40, 64)), CardBox(rect(112, 80))]),
            RegionSpec::Rects(vec![rounded(rect(40, 64)), rounded(rect(112, 80))])
        );
    }
}
