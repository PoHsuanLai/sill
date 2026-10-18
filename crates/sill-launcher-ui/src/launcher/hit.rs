//! Which part of the panel takes presses, measured by the panel and shared with the catcher
//! (the macOS polish pass: the palette's card is as tall as its content, so the 680 x 460 panel
//! is mostly transparent under a short list). The panel's input and blur follow the card; the
//! catcher's input leaves out only what the panel takes, so a press under a short card closes
//! the launcher like any other outside press.

use shell_host::{ElementId, ElementRegion, Rect, RegionSpec, RoundedRect};

use crate::launcher::layer::{
    CARD_BLUR_RADIUS, CARD_ID, MENU_BLUR_RADIUS, MENU_ID, PANEL_HEIGHT, PANEL_ROOM_WIDTH, SHADOW_SIDE,
};
use sill_ui_kit::links::PanelHit;

/// The panel's input region for `hit`: the card element itself (the host resolves it after
/// every layout), or the whole panel (never the shadow's room around it).
pub fn panel_input(hit: PanelHit) -> RegionSpec {
    match hit {
        PanelHit::Whole => {
            let side = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
            RegionSpec::Rects(vec![RoundedRect {
                rect: Rect {
                    x: side(SHADOW_SIDE),
                    y: 0,
                    width: side(PANEL_ROOM_WIDTH),
                    height: side(PANEL_HEIGHT),
                },
                radius: 0,
            }])
        }
        PanelHit::Card { .. } => card_region(),
    }
}

/// The card element, with the radius its squircle's shadows follow: the panel's blur region.
pub fn card_region() -> RegionSpec {
    RegionSpec::Element {
        id: ElementId::new(CARD_ID),
        radius: CARD_BLUR_RADIUS,
        inflate: 0,
    }
}

/// The panel's blur region: the card, and the actions menu too while it is open (it hangs below
/// the card, so without this quire paints its see-through tint over unblurred rows). The host
/// skips an element that is not in the document, so the menu's frame before it mounts blurs
/// only the card.
pub fn blur_region(actions: ActionsShown) -> RegionSpec {
    match actions {
        ActionsShown::Closed => card_region(),
        ActionsShown::Open => RegionSpec::Elements(vec![
            ElementRegion {
                id: ElementId::new(CARD_ID),
                radius: CARD_BLUR_RADIUS,
                inflate: 0,
            },
            ElementRegion {
                id: ElementId::new(MENU_ID),
                radius: MENU_BLUR_RADIUS,
                inflate: 0,
            },
        ]),
    }
}

/// What the panel takes, from the card's bottom edge in panel coordinates (`foot`) and whether
/// the actions menu is open. A foot at or above the top (read before layout) measures nothing.
pub fn hit_for(foot: f32, actions: ActionsShown) -> PanelHit {
    match actions {
        ActionsShown::Open => PanelHit::Whole,
        ActionsShown::Closed if foot >= 1.0 => PanelHit::Card {
            // A card is at most the panel's 460 px, so its rounded foot fits an i32.
            height: foot.ceil() as i32,
        },
        ActionsShown::Closed => PanelHit::Whole,
    }
}

/// Whether the selected row's actions menu is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionsShown {
    Open,
    Closed,
}

#[cfg(test)]
mod tests {
    use super::{ActionsShown, blur_region, card_region, hit_for, panel_input};
    use shell_host::{ElementId, Rect, RegionSpec};
    use sill_ui_kit::links::PanelHit;

    #[test]
    fn the_panel_takes_its_card_unless_the_actions_menu_may_hang_below_it() {
        const CASES: &[(f32, ActionsShown, PanelHit)] = &[
            (132.4, ActionsShown::Closed, PanelHit::Card { height: 133 }),
            (460.0, ActionsShown::Closed, PanelHit::Card { height: 460 }),
            (132.4, ActionsShown::Open, PanelHit::Whole),
            (0.0, ActionsShown::Closed, PanelHit::Whole),
        ];
        for &(foot, actions, want) in CASES {
            assert_eq!(hit_for(foot, actions), want, "{foot} {actions:?}");
        }
        let RegionSpec::Rects(whole) = panel_input(PanelHit::Whole) else {
            panic!("the whole panel is a rect");
        };
        let panel = Rect {
            x: 42,
            y: 0,
            width: 1040,
            height: 460,
        };
        assert_eq!(whole.iter().map(|r| r.rect).collect::<Vec<_>>(), [panel]);
        assert_eq!(panel_input(PanelHit::Card { height: 120 }), card_region());
    }

    #[test]
    fn the_blur_covers_the_card_and_adds_the_menu_only_while_it_is_open() {
        assert_eq!(blur_region(ActionsShown::Closed), card_region());
        let RegionSpec::Elements(open) = blur_region(ActionsShown::Open) else {
            panic!("an open menu unions two elements");
        };
        let ids: Vec<_> = open.iter().map(|e| e.id.clone()).collect();
        assert_eq!(
            ids,
            [ElementId::new("launcher-card"), ElementId::new("launcher-actions-menu")]
        );
    }
}
