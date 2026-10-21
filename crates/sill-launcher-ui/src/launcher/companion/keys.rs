//! The keys companion mode adds to the panel's (design/32 section 4.4), and the filter Tab
//! visits there. Pure and table-tested.
//!
//! Enter is the palette's own pick in every pane (the Ask row asks; an answer row's Enter is the
//! card's primary button), and Tab moves through the card's buttons as it does through any
//! row's actions. What is new is Escape's two meanings, command-Enter and command-period.

use prov::{RunId, TaskId};
use sill_launcher::ProviderKind;

use super::pane::Pane;
use crate::launcher::keys::Filter;

/// A key companion mode reads before the palette does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompanionKey {
    /// Escape.
    Escape,
    /// Command-Enter.
    CommandEnter,
    /// Command-period.
    CommandPeriod,
    /// The action that flips the conversation's permissions mode.
    SwitchMode,
}

/// What the selected row is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selected {
    /// No row.
    Nothing,
    /// The Ask row.
    Ask,
    /// An answer.
    Answer(TaskId),
    /// A run.
    Run(RunId),
    /// Any other row.
    Other,
}

/// What a key asks of the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionOut {
    /// Open the answer's object in its app, and close.
    Promote(TaskId),
    /// Stop the answer being made.
    Cancel(TaskId),
    /// Stop the selected run.
    StopRun(RunId),
    /// Flip the conversation between Auto and Manual.
    SwitchMode,
    /// Back to the prompt: the selection returns to the Ask row.
    Back,
    /// Put the companion away: the summon decides whether that closes the panel or restores the
    /// search it came from.
    Dismiss,
}

/// What `key` asks for in `pane` with `selected` under the selection.
pub fn companion_step(key: CompanionKey, pane: &Pane, selected: &Selected) -> Vec<CompanionOut> {
    match (key, pane) {
        (
            CompanionKey::SwitchMode,
            Pane::Composing
            | Pane::Thinking(_)
            | Pane::Showing(_)
            | Pane::Form(_)
            | Pane::Confirming(_),
        ) => vec![CompanionOut::SwitchMode],
        (CompanionKey::Escape, Pane::Composing) => vec![CompanionOut::Dismiss],
        (
            CompanionKey::Escape,
            Pane::Thinking(_) | Pane::Showing(_) | Pane::Form(_) | Pane::Confirming(_),
        ) => {
            vec![CompanionOut::Back]
        }
        (CompanionKey::CommandEnter, Pane::Showing(task) | Pane::Form(task)) => {
            vec![CompanionOut::Promote(task.clone())]
        }
        (CompanionKey::CommandEnter, Pane::Composing | Pane::Thinking(_) | Pane::Confirming(_)) => {
            Vec::new()
        }
        (CompanionKey::CommandPeriod, Pane::Thinking(task) | Pane::Confirming(task)) => {
            vec![CompanionOut::Cancel(task.clone())]
        }
        (CompanionKey::CommandPeriod, Pane::Composing) => match selected {
            Selected::Run(run) => vec![CompanionOut::StopRun(run.clone())],
            Selected::Nothing | Selected::Ask | Selected::Answer(_) | Selected::Other => Vec::new(),
        },
        (CompanionKey::CommandPeriod, Pane::Showing(_) | Pane::Form(_)) => Vec::new(),
    }
}

/// The filters Tab visits in companion mode, in order: the companion's own rows, then the
/// helpers, every run, and the memory timeline. There is no "every provider": the search is not
/// here.
const RING: [ProviderKind; 4] = [
    ProviderKind::Companion,
    ProviderKind::Helpers,
    ProviderKind::Runs,
    ProviderKind::Memory,
];

/// Whether the rows of `filter` follow the companion's state: they are asked for again whenever
/// an answer, a run or the journal moves. The memory timeline does not: asking for it moves the
/// state itself (the ask is published as it goes out), so following it would ask again at once,
/// without end. It is asked again only when an answer to a waiting memory comes back
/// (`PanelCtx::memory_refresh`).
pub fn follows_state(filter: Filter) -> bool {
    filter != Filter::Only(ProviderKind::Memory)
}

/// The filter after `filter` going forward (`Forward`) or back, wrapping within [`RING`]; a
/// filter outside it starts the ring over.
pub fn companion_filter(filter: Filter, way: Way) -> Filter {
    let at = match filter {
        Filter::Only(kind) => RING.iter().position(|k| *k == kind),
        Filter::All => None,
    };
    let next = match (at, way) {
        (None, Way::Forward) => 0,
        (None, Way::Backward) => RING.len() - 1,
        (Some(at), Way::Forward) => (at + 1) % RING.len(),
        (Some(at), Way::Backward) => (at + RING.len() - 1) % RING.len(),
    };
    Filter::Only(RING[next])
}

/// Which way Tab goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    /// Tab.
    Forward,
    /// Shift+Tab.
    Backward,
}
