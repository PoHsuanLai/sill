//! The pure transition: which computers are asking after an event, and what to tell and answer.

use super::command::{GuestAsksEffect, GuestAsksEvent};
use super::model::GuestAsksState;

/// `state` after `event`, with the effects it calls for. A repeat of a question already told does
/// nothing; a press for a computer not asking (answered elsewhere meanwhile) does nothing.
pub fn step(
    state: GuestAsksState,
    event: GuestAsksEvent,
) -> (GuestAsksState, Vec<GuestAsksEffect>) {
    let mut asks = state.asks;
    let effects = match event {
        GuestAsksEvent::Asked(ask) => match asks.contains(&ask) {
            true => Vec::new(),
            false => {
                match asks.iter_mut().find(|known| known.node == ask.node) {
                    Some(known) => known.clone_from(&ask),
                    None => asks.push(ask.clone()),
                }
                vec![GuestAsksEffect::Tell(ask)]
            }
        },
        GuestAsksEvent::Waiting(waiting) => {
            let gone = asks
                .iter()
                .filter(|known| waiting.iter().all(|ask| ask.node != known.node))
                .map(|known| GuestAsksEffect::Withdraw(known.node.clone()));
            let new = waiting
                .iter()
                .filter(|ask| !asks.contains(ask))
                .cloned()
                .map(GuestAsksEffect::Tell);
            let effects = gone.chain(new).collect();
            asks = waiting;
            effects
        }
        GuestAsksEvent::Chosen(node, verdict) => {
            let before = asks.len();
            asks.retain(|known| known.node != node);
            match asks.len() < before {
                true => vec![
                    GuestAsksEffect::Withdraw(node.clone()),
                    GuestAsksEffect::Answer(node, verdict),
                ],
                false => Vec::new(),
            }
        }
    };
    (GuestAsksState { asks }, effects)
}
