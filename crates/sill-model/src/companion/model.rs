//! What the companion service publishes: whether it reaches the companion, the machines that
//! decide where a summon goes, the answers the companion has made, the computer-use runs, the
//! undo journal, and the answers to the launcher's two lookups.

use std::collections::{BTreeMap, BTreeSet};

use almanac_core::{FactView, TimelineEntry};
use companion_wire::{AnswerWire, RouteNote};
use cua_bus::{BudgetBody, CuaOutcome, RunStateSlug};
use docket_core::{ActionRef, CardActionId, Handle, Hit, RunMode, UndoEntry};
use prov::{AppName, EntityKind, RunId, SessionId, TaskId};
use sill_launcher::Generation;

use super::helpers::HelpersModel;
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
    /// The router session the answer's object names, read once when it appeared: a handle
    /// resolves only in the session that holds it. `None` until it is read.
    pub session: Option<SessionId>,
    /// How the answer's last model turn was reached, as its `Routing` property says: empty until
    /// a model announced a stage (and for a companion without the property).
    pub routing: Vec<RouteNote>,
    /// The text behind each handle of the answer, as the router showed it to the screen. It
    /// is for drawing only: nothing here is ever sent back to a model.
    pub shown: BTreeMap<Handle, String>,
}

/// Whether a card's button is waiting for its call to end.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Busy {
    /// No button was pressed, or its step ended.
    #[default]
    Idle,
    /// This button was pressed and its step has not ended: the companion's plan for the answer
    /// shows `step` pending or running (or does not show it yet).
    Acting {
        /// The button.
        card: CardActionId,
        /// The action the button calls, which names its step in the answer's plan.
        step: ActionRef,
    },
    /// The companion would not take this button's press.
    Refused(CardActionId),
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

/// A kind of thing an app opens in its own window when a search row of it is picked: the app
/// declares the kind's `<kind>.open` action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Opener {
    /// The app.
    pub app: AppName,
    /// The kind of thing.
    pub kind: EntityKind,
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
    /// The kinds of thing the installed apps open in their own windows.
    pub openers: BTreeSet<Opener>,
    /// What the things provider asked for.
    pub things: Lookup<Hit>,
    /// What the memory provider asked for.
    pub memory: Lookup<TimelineEntry>,
    /// What assistants asked to remember and the person has not answered, read with the
    /// timeline under the same generation.
    pub waiting: Lookup<FactView>,
    /// A plain sentence when answering one of them did not go through.
    pub memory_notice: Option<String>,
    /// The provider rows' labels for coding-agent programs; empty until porter's list is read.
    pub programs: super::ProgramLabels,
    /// The helpers the person keeps.
    pub helpers: HelpersModel,
}
