//! A card's buttons.

use companion_wire::CardWire;
use ds::components::companion::answer::action::{ActionRole, CardAction, CardActionId};
use ds_core::vocab::Availability;
use prov::Effect;

use super::marks::effect_mark;

/// A card's button. The first button that only reads or writes undoably is the default; an
/// outbound button is never drawn as the default without its mark, and a destructive one is
/// never the default at all.
pub fn card_action(card: &CardWire, first: bool) -> CardAction {
    let role = match (card.effect, first) {
        (Effect::Destructive, _) => ActionRole::Destructive,
        (Effect::Read | Effect::UndoableWrite | Effect::Outbound, true) => ActionRole::Primary,
        (Effect::Read | Effect::UndoableWrite | Effect::Outbound, false) => ActionRole::Secondary,
    };
    CardAction {
        id: CardActionId(card.id.as_str().to_owned()),
        label: card.label.as_str().to_owned(),
        role,
        effect: effect_mark(card.effect),
        keys: None,
        availability: Availability::Enabled,
    }
}

/// A card's buttons, the first of them the default.
pub(super) fn actions(cards: &[CardWire]) -> Vec<CardAction> {
    cards
        .iter()
        .enumerate()
        .map(|(at, card)| card_action(card, at == 0))
        .collect()
}
