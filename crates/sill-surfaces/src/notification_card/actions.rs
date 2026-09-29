//! A card's actions (design/13 §13.3.6 "Hover"): every action but `default` (the card's own
//! press) as quire's `CardAction`, which the card draws as Mini buttons that open on hover and
//! keep their press.

use dioxus::prelude::*;
use ds::{CardAction, TextLine};
use sill_model::notifications::{Action, Notification};

use super::CardPress;

/// `note`'s buttons, each telling `on_press` its key.
pub fn card_actions(note: &Notification, on_press: EventHandler<CardPress>) -> Vec<CardAction> {
    note.buttons()
        .cloned()
        .map(|action| card_action(action, on_press))
        .collect()
}

fn card_action(action: Action, on_press: EventHandler<CardPress>) -> CardAction {
    let Action { key, label } = action;
    CardAction {
        label: TextLine::from(label),
        on_press: EventHandler::new(move |_| on_press.call(CardPress::Action(key.clone()))),
    }
}
