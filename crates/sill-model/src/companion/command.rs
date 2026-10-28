//! What tells the companion service things, what happens in it, and what that asks of the
//! outside.

use almanac_core::{FactId, FactView, MemoryItem, TimelineEntry};
use companion_wire::{AnswerWire, RouteNote};
use docket_core::{ContextKeep, Hit, TurnVia, UndoEntry, UndoScope};
use docket_permissions_core::Mode;
use docket_vocab::{CallRequest, CardActionId, Handle};
use docket_wire::{SummonAnswer, SummonSerial};
use ds_core::vocab::MemoryVerb;
use prov::{AgentRef, AppName, RunId, SessionId, SpaceId, TaskId};
use sill_launcher::{Generation, LauncherMode, RunControl, WindowKey};

use std::collections::BTreeSet;

use super::helpers::{HelpersCommand, HelpersMsg};
use super::mode::ModeMsg;
use super::model::{Opener, RunFacts};
use super::sight::SightState;
use super::summon::{FocusFacts, SummonOut, SummonParams};
use super::tap::TapParams;
use super::terminals::{TerminalsCommand, TerminalsMsg};

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
    /// The panel's mode while it is showing; `None` when it is not.
    pub launcher: Option<LauncherMode>,
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
    /// The Space the ask is made in: the focused workspace's, or `desktop` outside any.
    pub space: SpaceId,
    /// Whether the words were typed or spoken through the hold gesture.
    pub via: TurnVia,
}

/// What the daemon and the surfaces tell the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionCommand {
    /// Switch a conversation the box may not hold to `mode`, for a probe (`sill debug mode`): the
    /// router is asked as for the box's own, and only a change it signals reaches the box.
    SessionMode {
        /// The conversation.
        session: SessionId,
        /// The mode it is to be in.
        mode: docket_permissions_core::Mode,
    },
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
    /// Say something to a subagent: the router delivers it, then the companion is told.
    Tell {
        /// Who to say it to.
        agent: AgentRef,
        /// The Space it works in.
        space: SpaceId,
        /// The person's words.
        text: String,
    },
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
    /// Read the memory timeline and what waits for the person; the answers land in `memory` and
    /// `waiting` under `generation`.
    Timeline {
        /// Which ask.
        generation: Generation,
    },
    /// Read the undo journal again.
    RefreshJournal,
    /// Perform an action of an app as the person asked for it (a thing's primary action).
    Perform(Box<CallRequest>),
    /// Something the person did on the helpers screen.
    Helpers(HelpersMsg),
    /// Something the person did to a session in a terminal.
    Terminals(TerminalsMsg),
    /// The person chose or flipped the conversation's mode.
    Mode(ModeMsg),
}

/// What happens in the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionEvent {
    /// The service's place on the bus changed.
    Reached(bool),
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
        /// The router session the ask was made in.
        session: SessionId,
    },
    /// An answer appeared or changed.
    Answer(Box<AnswerWire>),
    /// An answer's object went away.
    AnswerGone(TaskId),
    /// A card button was pressed: its call starts.
    CardPressed {
        /// The answer.
        task: TaskId,
        /// Which button.
        action: CardActionId,
    },
    /// The companion would not take a card button's press.
    CardRefused {
        /// The answer.
        task: TaskId,
        /// Which button.
        action: CardActionId,
    },
    /// The router showed the text behind a handle of an answer.
    Shown {
        /// The answer.
        task: TaskId,
        /// The handle.
        handle: Handle,
        /// What it stands for, for the screen alone.
        text: String,
    },
    /// The router refused to show a handle of an answer (its session is closed, or never held
    /// it): the answer shows the handle's plain fallback text and the router is not asked again.
    Unshown {
        /// The answer.
        task: TaskId,
        /// The handle.
        handle: Handle,
    },
    /// The answer object's router session: its `Session` property, or `None` when the object
    /// has none (an older companion).
    Session {
        /// The answer.
        task: TaskId,
        /// The session its handles are shown through.
        session: Option<SessionId>,
    },
    /// The answer object's `Routing` property, read as it appeared and again when it changed.
    Routing {
        /// The answer.
        task: TaskId,
        /// The stages of its last model turn.
        notes: Vec<RouteNote>,
    },
    /// A run appeared or changed.
    Run(Box<RunFacts>),
    /// A run's object went away.
    RunGone(RunId),
    /// What cuad says about the screen model, as it appeared and again when it changed.
    Sight(SightState),
    /// The undo journal, newest first.
    Journal(Vec<UndoEntry>),
    /// The installed manifests changed: these kinds of thing open in their apps.
    Openers(BTreeSet<Opener>),
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
    /// What assistants asked to remember, read with the timeline.
    WaitingLoaded(Generation, Vec<FactView>),
    /// That list could not be read.
    WaitingFailed(Generation),
    /// The person's answer about this one was taken: it no longer waits.
    Settled(FactId),
    /// The answer about one could not be taken.
    SettleFailed,
    /// Something about the helpers: the person's doing, or what a call to the companion came to.
    Helpers(Box<HelpersMsg>),
    /// Something about the sessions in terminals: the person's doing, or what the companion said.
    Terminals(TerminalsMsg),
    /// Something about the conversation's mode: the person's switch, what a session opened in,
    /// or what the router said.
    Mode(ModeMsg),
}

/// What the service must do outside the pure state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanionEffect {
    /// A summon's machine asked for something: the daemon opens, switches or closes the
    /// launcher; the service asks the app.
    Summon(SummonOut),
    /// Warm the planner model: a prompt is about to be typed.
    Prepare,
    /// The helpers screen asks the companion for something.
    Helpers(HelpersCommand),
    /// The panel asks the companion about the sessions in terminals.
    Terminals(TerminalsCommand),
    /// Ask the router to put `session` in `mode`. The answer is a [`ModeMsg`] event.
    SetMode {
        /// The session.
        session: SessionId,
        /// The mode to switch to.
        mode: Mode,
    },
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
