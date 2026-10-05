//! What tells the companion service things, what happens in it, and what that asks of the
//! outside.

use almanac_core::{MemoryItem, TimelineEntry};
use companion_wire::AnswerWire;
use docket_core::{
    CallRequest, CardActionId, ContextKeep, Hit, SummonAnswer, SummonSerial, UndoEntry, UndoScope,
};
use ds_core::vocab::MemoryVerb;
use prov::{AppName, RunId, TaskId};
use sill_launcher::{Generation, LauncherMode, RunControl, WindowKey};

use super::model::{CompanionLink, RunFacts};
use super::summon::{FocusFacts, LauncherFacts, SummonOut, SummonParams};
use super::tap::TapParams;

/// What the settings say about the companion's timing: the double tap's window, how long an
/// app has to take a summon and where a summon in a field goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompanionParams {
    /// The double tap.
    pub tap: TapParams,
    /// The summon.
    pub summon: SummonParams,
}

/// What the person has focused when a tap or a summon arrives; the service finds out whether the
/// app serves the companion before the machines see it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FocusInput {
    /// The focused window.
    pub window: Option<WindowKey>,
    /// Its app, if it has a bus name.
    pub app: Option<AppName>,
    /// The panel.
    pub launcher: LauncherFacts,
}

/// A prompt the person sent from the launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskInput {
    /// What they typed.
    pub prompt: String,
    /// The chips of context they kept.
    pub keep: ContextKeep,
    /// The window they asked from.
    pub window: Option<WindowKey>,
    /// The app they summoned the companion from; none for the launcher alone.
    pub app: Option<AppName>,
}

/// What the daemon and the surfaces tell the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionCommand {
    /// The settings changed.
    Params(CompanionParams),
    /// The double-tap key was tapped.
    Tap(FocusInput),
    /// The companion was summoned.
    Summon(FocusInput),
    /// Put the companion away.
    Dismiss,
    /// The panel closed.
    LauncherClosed,
    /// Pause every run that is going.
    StopAll,
    /// Send a prompt.
    Ask(AskInput),
    /// Press a button on an answer's card.
    Card {
        /// The answer.
        task: TaskId,
        /// Which button.
        action: CardActionId,
    },
    /// Stop an answer's task.
    Cancel(TaskId),
    /// Open an answer's object in its app: its card's open button, when it has one.
    Promote(TaskId),
    /// Steer a run.
    Run {
        /// The run.
        run: RunId,
        /// How.
        control: RunControl,
    },
    /// Undo what was done.
    Undo(UndoScope),
    /// Do something to a memory row.
    Memory {
        /// The row.
        item: MemoryItem,
        /// What.
        verb: MemoryVerb,
    },
    /// Search the apps' indexes for `text`; the answer lands in `things` under `generation`.
    Things {
        /// What to find.
        text: String,
        /// Which keystroke asked.
        generation: Generation,
    },
    /// Read the memory timeline; the answer lands in `memory` under `generation`.
    Timeline {
        /// Which ask.
        generation: Generation,
    },
    /// Read the undo journal again.
    RefreshJournal,
    /// Perform an action of an app as the person asked for it (a thing's primary action).
    Perform(CallRequest),
}

/// What happens in the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionEvent {
    /// The service's place on the bus changed.
    Link(CompanionLink),
    /// The double-tap key was tapped, with what was focused.
    Tapped(FocusFacts),
    /// A summon arrived, with what was focused.
    Summoned(FocusFacts),
    /// An app answered the summon it was asked.
    AppAnswered(SummonSerial, SummonAnswer),
    /// A wait the machines asked for has passed.
    Elapsed,
    /// The panel closed.
    LauncherClosed,
    /// The companion was put away.
    Dismissed,
    /// An ask began: its answer object exists.
    Asked {
        /// The task.
        task: TaskId,
        /// What was typed.
        prompt: String,
    },
    /// An answer appeared or changed.
    Answer(AnswerWire),
    /// An answer's object went away.
    AnswerGone(TaskId),
    /// A card button was pressed: its call is running.
    CardPressed {
        /// The answer.
        task: TaskId,
        /// Which button.
        action: CardActionId,
    },
    /// A card button's call ended.
    CardEnded(TaskId),
    /// A run appeared or changed.
    Run(Box<RunFacts>),
    /// A run's object went away.
    RunGone(RunId),
    /// The undo journal, newest first.
    Journal(Vec<UndoEntry>),
    /// A search of the apps' indexes was sent.
    ThingsAsked(Generation),
    /// The indexed hits of a search.
    ThingsFound(Generation, Vec<Hit>),
    /// Hits an app answered late.
    ThingsLate(Generation, Vec<Hit>),
    /// A search could not be answered.
    ThingsFailed(Generation),
    /// A timeline read was sent.
    TimelineAsked(Generation),
    /// The timeline's page.
    TimelineLoaded(Generation, Vec<TimelineEntry>),
    /// The timeline could not be read.
    TimelineFailed(Generation),
}

/// What the service must do outside the pure state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionEffect {
    /// A summon's machine asked for something: the daemon opens, switches or closes the
    /// launcher; the service asks the app.
    Summon(SummonOut),
    /// Warm the planner model: a prompt is about to be typed.
    Prepare,
}

impl SummonOut {
    /// The mode a launcher opens or switches into, if this opens one: what warms the model.
    pub fn opens_prompt(&self) -> bool {
        match self {
            SummonOut::AskApp { .. }
            | SummonOut::OpenLauncher {
                mode: LauncherMode::Companion,
                ..
            }
            | SummonOut::SwitchLauncher(LauncherMode::Companion) => true,
            SummonOut::OpenLauncher { .. }
            | SummonOut::SwitchLauncher(LauncherMode::Search)
            | SummonOut::CloseLauncher => false,
        }
    }
}
