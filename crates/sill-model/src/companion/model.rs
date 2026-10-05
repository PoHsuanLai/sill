//! What the companion service publishes: whether it reaches the companion, the machines that
//! decide where a summon goes, the answers the companion has made, the computer-use runs, the
//! undo journal, and the answers to the launcher's two lookups.

use std::collections::BTreeMap;

use almanac_core::TimelineEntry;
use companion_wire::AnswerWire;
use cua_bus::{BudgetBody, CuaOutcome, RunStateSlug};
use docket_core::{CardActionId, Hit, RunMode, UndoEntry};
use prov::{RunId, TaskId};
use sill_launcher::Generation;

use super::summon::Summon;
use super::tap::Tap;

/// Whether the service reaches the companion's bus names.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CompanionLink {
    /// Not on the bus yet: the shell's seat at intentd has not claimed its names.
    #[default]
    Passive,
    /// Connected as the shell.
    Ready,
}

/// One answer, as the companion has it, with the words that asked for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerFacts {
    /// What the person typed, for the answer's row.
    pub prompt: String,
    /// The answer as the companion last wrote it.
    pub wire: AnswerWire,
    /// What the person is waiting on from a card button.
    pub busy: Busy,
}

/// Whether a card's button is waiting for its call to end.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Busy {
    /// No button was pressed, or its call ended.
    #[default]
    Idle,
    /// This button's call is running.
    Acting(CardActionId),
}

/// One computer-use run, as cuad publishes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFacts {
    /// The run.
    pub run: RunId,
    /// What it was asked to do.
    pub goal: String,
    /// The app whose window it works in, as the window's title says.
    pub window: Option<String>,
    /// How many steps it has taken.
    pub step: u32,
    /// Its phase.
    pub state: RunStateSlug,
    /// How it ended, once it has.
    pub outcome: Option<CuaOutcome>,
    /// How it reaches its window.
    pub mode: RunMode,
    /// Some of the model's last thought, as data.
    pub thought: Option<String>,
    /// Its limits and what it has used.
    pub budget: Option<BudgetBody>,
}

/// How far a lookup has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LookupPhase {
    /// Nothing asked.
    #[default]
    Idle,
    /// Asked; the answer has not come.
    Asked,
    /// Answered.
    Answered,
    /// The peer could not answer.
    Failed,
}

/// The answer to the newest lookup of one kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lookup<T> {
    /// Which ask this answers: a newer ask replaces it.
    pub generation: Generation,
    /// How far it has got.
    pub phase: LookupPhase,
    /// What was found, in the peer's order.
    pub found: Vec<T>,
}

impl<T> Default for Lookup<T> {
    fn default() -> Lookup<T> {
        Lookup {
            generation: Generation::default(),
            phase: LookupPhase::Idle,
            found: Vec::new(),
        }
    }
}

/// Everything the companion service publishes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Companion {
    /// Whether the companion's bus is reached.
    pub link: CompanionLink,
    /// The double tap.
    pub tap: Tap,
    /// Where a summon is.
    pub summon: Summon,
    /// The serial of the last ask made of an app; the next is one more.
    pub asked_serial: u64,
    /// The answers, by task.
    pub answers: BTreeMap<TaskId, AnswerFacts>,
    /// The task the launcher returns to.
    pub front: Option<TaskId>,
    /// The runs cuad has, by run.
    pub runs: BTreeMap<RunId, RunFacts>,
    /// The undo journal, newest first.
    pub journal: Vec<UndoEntry>,
    /// What the things provider asked for.
    pub things: Lookup<Hit>,
    /// What the memory provider asked for.
    pub memory: Lookup<TimelineEntry>,
}
