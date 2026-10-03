//! The seat's one transition.

use docket_core::ConfirmAnswer;

use super::command::{IntentsEffect, IntentsEvent};
use super::model::{Intents, IntentsLink};
use super::sheet::{Reply, SheetEffect, SheetEvent, step as sheet_step};
use super::terminal::step as terminal_step;

/// `state` after `event`, and what the outside must do.
pub fn step(state: Intents, event: IntentsEvent) -> (Intents, Vec<IntentsEffect>) {
    match event {
        IntentsEvent::Link(link) => (relinked(state, link), Vec::new()),
        IntentsEvent::Sheet(event) => {
            let (sheets, effects) = sheet_step(state.sheets, event);
            let granted = effects.iter().any(grants_the_terminal);
            let mut effects: Vec<IntentsEffect> =
                effects.into_iter().map(IntentsEffect::Sheet).collect();
            if granted {
                effects.push(IntentsEffect::ReadGrantsSoon);
            }
            (Intents { sheets, ..state }, effects)
        }
        IntentsEvent::Terminal(event) => (
            Intents {
                terminal: terminal_step(state.terminal, event),
                ..state
            },
            Vec::new(),
        ),
    }
}

/// A link that is not serving has no questions in flight: the asker was never there, or went.
fn relinked(state: Intents, link: IntentsLink) -> Intents {
    let sheets = match link {
        IntentsLink::Serving => state.sheets,
        IntentsLink::Passive | IntentsLink::Unavailable(_) => {
            sheet_step(state.sheets, SheetEvent::AskerGone).0
        }
    };
    Intents {
        link,
        sheets,
        ..state
    }
}

/// Whether the effect answers "allow from the terminal".
fn grants_the_terminal(effect: &SheetEffect) -> bool {
    matches!(
        effect,
        SheetEffect::Respond(_, Reply::Answer(ConfirmAnswer::AllowedFromTerminal { .. }))
    )
}
