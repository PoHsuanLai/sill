//! The pure transition: which holds are on after an event, and what the person is to be told.

use super::command::{SyncHoldEffect, SyncHoldEvent};
use super::model::SyncHoldState;

/// `state` after `event`, with the effects it calls for. A repeat for a dataset replaces its
/// hold and its notification; the end of a hold nobody was told about does nothing.
pub fn step(state: SyncHoldState, event: SyncHoldEvent) -> (SyncHoldState, Vec<SyncHoldEffect>) {
    let mut holds = state.holds;
    let effects = match event {
        SyncHoldEvent::Held(hold) => {
            match holds.iter_mut().find(|known| known.dataset == hold.dataset) {
                Some(known) => known.clone_from(&hold),
                None => holds.push(hold.clone()),
            }
            vec![SyncHoldEffect::Tell(hold)]
        }
        SyncHoldEvent::Ended(dataset) => {
            let before = holds.len();
            holds.retain(|known| known.dataset != dataset);
            match holds.len() < before {
                true => vec![SyncHoldEffect::Withdraw(dataset)],
                false => Vec::new(),
            }
        }
    };
    (SyncHoldState { holds }, effects)
}
