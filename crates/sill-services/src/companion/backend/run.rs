//! The service's loop. Commands become bus calls (each on its own task, answering through the
//! event channel); events and the machines' wakes go through the pure `step`, whose state is
//! published and whose effects are carried out here: the summon's launcher effects to the
//! daemon, the ask to the app, the warm-up to the companion.
//!
//! The seat arrives and leaves with the intents service. While it is held the loop follows the
//! companion's answers, cuad's runs and intentd's journal and late hits; while it is not, the
//! machines still run and every lookup answers `Failed`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use docket_core::{CardActionId, Handle, UndoScope};
use docket_dbus::{COMPANION_BUS, CompanionProxy, INTENTS_BUS, answer_path};
use ds_core::time::stamp::Stamp;
use prov::{SessionId, TaskId, UnixSeconds};
use sill_launcher::{Generation, RunControl};
use sill_model::companion::{
    Busy, Companion, CompanionCommand, CompanionEffect, CompanionEvent, CompanionLink,
    CompanionParams, SummonOut, card_call, going_runs, promote_card, step, unresolved_handles,
    wake,
};
use sill_model::launch::{LaunchCommand, SettingsPage};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use super::probe::{self, Arrival};
use super::text::text;
use super::{answer, cua, intents, memory, tell};
use crate::companion::{CompanionLinks, CompanionService};
use crate::control::Control;
use crate::env::{Env, WallClock};
use crate::owner;
use crate::seat::SeatView;
use crate::service::ServiceIo;
use crate::stream::next;

/// What the loop reaches outside the process: the wall clock the turns are stamped with.
pub(crate) struct Reach {
    clock: Arc<dyn WallClock>,
}

impl Reach {
    pub(crate) fn of(env: &Env) -> Reach {
        Reach {
            clock: Arc::clone(&env.clock),
        }
    }
}

/// What the loop's tasks tell it besides events.
enum Inner {
    /// An answer object appeared (or was asked for): follow it.
    Follow(OwnedObjectPath),
    /// An answer object went away.
    Unfollow(OwnedObjectPath),
}

pub(crate) async fn run(reach: Reach, links: CompanionLinks, io: ServiceIo<CompanionService>) {
    let ServiceIo {
        state: tx,
        mut commands,
    } = io;
    let (inner_tx, mut inner) = mpsc::unbounded_channel();
    let (events_tx, mut events) = mpsc::unbounded_channel();
    let (arrivals_tx, arrivals) = mpsc::unbounded_channel();
    let CompanionLinks {
        seat,
        summons,
        launch,
    } = links;
    tokio::spawn(probe::run(seat.clone(), arrivals, events_tx.clone()));
    let mut seat = Some(seat);
    let mut live = Live {
        state: Companion::default(),
        tx,
        params: CompanionParams::default(),
        origin: Instant::now(),
        clock: reach.clock,
        connection: None,
        summons,
        launch,
        inner: inner_tx,
        events: events_tx,
        arrivals: arrivals_tx,
        followed: HashMap::new(),
        watchers: Vec::new(),
        displayed: HashSet::new(),
        old_companion: OldCompanion::Unremarked,
        opened: HashMap::new(),
    };
    live.seated(seat.as_ref().and_then(SeatView::current));
    loop {
        let due = wake(&live.state);
        let sleeper = due.map(|at| live.origin + Duration::from_millis(at.0));
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command),
                None => break,
            },
            Some(event) = events.recv() => live.feed(event),
            Some(event) = inner.recv() => live.inner(event),
            changed = async {
                match seat.as_mut() {
                    Some(seat) => seat.changed().await,
                    None => std::future::pending().await,
                }
            } => {
                if changed.is_err() {
                    seat = None;
                    live.seated(None);
                } else {
                    live.seated(seat.as_ref().and_then(SeatView::current));
                }
            }
            () = async {
                match sleeper {
                    Some(at) => tokio::time::sleep_until(at).await,
                    None => std::future::pending().await,
                }
            } => live.feed(CompanionEvent::Elapsed),
        }
    }
    live.seated(None);
}

struct Live {
    state: Companion,
    tx: watch::Sender<Companion>,
    params: CompanionParams,
    /// Where the machines' stamps count from.
    origin: Instant,
    clock: Arc<dyn WallClock>,
    connection: Option<Connection>,
    summons: mpsc::UnboundedSender<SummonOut>,
    launch: Control<LaunchCommand>,
    inner: mpsc::UnboundedSender<Inner>,
    events: mpsc::UnboundedSender<CompanionEvent>,
    arrivals: mpsc::UnboundedSender<Arrival>,
    followed: HashMap<String, JoinHandle<()>>,
    watchers: Vec<JoinHandle<()>>,
    /// The handles the router was already asked to show, so a failed one is not asked again on
    /// every update of its answer.
    displayed: HashSet<(TaskId, Handle)>,
    old_companion: OldCompanion,
    /// The session each answer sill asked was opened in: the fallback for a companion whose
    /// answers do not name theirs.
    opened: HashMap<TaskId, SessionId>,
}

/// Whether the log already says an answer came without its `Session` property.
enum OldCompanion {
    Unremarked,
    Remarked,
}

impl Live {
    fn now(&self) -> Stamp {
        Stamp(u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX))
    }

    fn unix(&self) -> UnixSeconds {
        UnixSeconds(self.clock.now().timestamp().as_second())
    }

    /// Step the state by `event` now, publish it when it changed, and carry out what it asks.
    fn feed(&mut self, event: CompanionEvent) {
        let Some(event) = self.named(event) else {
            return;
        };
        if let CompanionEvent::Asked { task, session, .. } = &event {
            self.opened.insert(task.clone(), session.clone());
        }
        let answered = match &event {
            CompanionEvent::Answer(wire) => Some(wire.task.clone()),
            CompanionEvent::Asked { task, .. } | CompanionEvent::Session { task, .. } => {
                Some(task.clone())
            }
            _ => None,
        };
        if let CompanionEvent::AnswerGone(task) = &event {
            self.displayed.retain(|(shown, _)| shown != task);
            self.opened.remove(task);
        }
        let (next, effects) = step(self.state.clone(), event, self.now(), &self.params);
        if next != self.state {
            self.tx.send_replace(next.clone());
        }
        self.state = next;
        for effect in effects {
            self.carry_out(effect);
        }
        if let Some(task) = answered {
            self.show_handles(&task);
        }
    }

    /// An answer object that names no session is from an older companion: it is shown through
    /// the session the ask opened, if sill asked it; otherwise it has none.
    fn named(&mut self, event: CompanionEvent) -> Option<CompanionEvent> {
        let CompanionEvent::Session {
            task,
            session: None,
        } = event
        else {
            return Some(event);
        };
        self.warn_old_companion();
        let session = self.opened.get(&task).cloned()?;
        Some(CompanionEvent::Session {
            task,
            session: Some(session),
        })
    }

    /// Say once that an answer came without its `Session` property.
    fn warn_old_companion(&mut self) {
        if matches!(self.old_companion, OldCompanion::Unremarked) {
            eprintln!(
                "sill: companion: an answer names no Session; using the session the ask opened"
            );
            self.old_companion = OldCompanion::Remarked;
        }
    }

    /// Ask the router to show the text behind each handle of `task`'s answer that has not been
    /// asked for: the card draws it, and nothing else reads it.
    fn show_handles(&mut self, task: &TaskId) {
        let Some(answer) = self.state.answers.get(task) else {
            return;
        };
        let (Some(session), Some(connection)) = (answer.session.clone(), self.connection.clone())
        else {
            return;
        };
        for handle in unresolved_handles(answer) {
            if !self.displayed.insert((task.clone(), handle)) {
                continue;
            }
            let (session, connection, events, task) = (
                session.clone(),
                connection.clone(),
                self.events.clone(),
                task.clone(),
            );
            tokio::spawn(async move {
                match intents::display(&connection, session, handle).await {
                    Ok(text) => {
                        let _ = events.send(CompanionEvent::Shown { task, handle, text });
                    }
                    Err(why) => {
                        eprintln!("sill: companion: showing a handle: {why}");
                        let _ = events.send(CompanionEvent::Unshown { task, handle });
                    }
                }
            });
        }
    }

    fn carry_out(&mut self, effect: CompanionEffect) {
        match effect {
            CompanionEffect::Prepare => {
                self.on_bus("warming the planner", |connection| async move {
                    answer::prepare(&connection).await
                });
            }
            CompanionEffect::PickModel => {
                let page = LaunchCommand::OpenSettingsPage(SettingsPage::AssistantModel);
                if let Err(error) = self.launch.send(page) {
                    eprintln!("sill: companion: opening the model picker: {error}");
                }
            }
            CompanionEffect::Summon(SummonOut::AskApp { app, serial }) => {
                let events = self.events.clone();
                match self.connection.clone() {
                    Some(connection) => {
                        tokio::spawn(async move {
                            let _ = events.send(probe::ask_app(connection, app, serial).await);
                        });
                    }
                    None => {
                        let _ = events.send(CompanionEvent::AppAnswered(
                            serial,
                            docket_core::SummonAnswer::Declined,
                        ));
                    }
                }
            }
            CompanionEffect::Summon(out) => {
                let _ = self.summons.send(out);
            }
        }
    }

    /// Run `call` on its own task when the seat is held; log what it says when it fails.
    fn on_bus<F, Fut>(&self, what: &'static str, call: F)
    where
        F: FnOnce(Connection) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        let Some(connection) = self.connection.clone() else {
            return;
        };
        tokio::spawn(async move {
            if let Err(why) = call(connection).await {
                eprintln!("sill: companion: {what}: {why}");
            }
        });
    }

    /// The seat was taken (`Some`) or given up (`None`).
    fn seated(&mut self, connection: Option<Connection>) {
        for watcher in self.watchers.drain(..) {
            watcher.abort();
        }
        for (_, follower) in self.followed.drain() {
            follower.abort();
        }
        self.connection = connection.clone();
        match connection {
            None => self.feed(CompanionEvent::Link(CompanionLink::Passive)),
            Some(connection) => {
                self.feed(CompanionEvent::Link(CompanionLink::Ready));
                self.watch(&connection);
            }
        }
    }

    /// Start following what the companion, cuad and intentd say.
    fn watch(&mut self, connection: &Connection) {
        self.watchers.push(tokio::spawn(cua::follow(
            connection.clone(),
            self.events.clone(),
        )));
        self.watchers.push(tokio::spawn(follow_answers(
            connection.clone(),
            self.inner.clone(),
        )));
        self.watchers.push(tokio::spawn(follow_intentd(
            connection.clone(),
            self.events.clone(),
        )));
    }

    fn inner(&mut self, inner: Inner) {
        match inner {
            Inner::Follow(path) => {
                let key = path.to_string();
                if let (Some(connection), false) =
                    (self.connection.clone(), self.followed.contains_key(&key))
                {
                    let handle =
                        tokio::spawn(answer::follow(connection, path, self.events.clone()));
                    self.followed.insert(key, handle);
                }
            }
            Inner::Unfollow(path) => {
                if let Some(handle) = self.followed.remove(path.as_str()) {
                    handle.abort();
                }
                let gone = self
                    .state
                    .answers
                    .keys()
                    .find(|task| answer_path(task) == path.as_str())
                    .cloned();
                if let Some(task) = gone {
                    self.feed(CompanionEvent::AnswerGone(task));
                }
            }
        }
    }

    fn command(&mut self, command: CompanionCommand) {
        match command {
            CompanionCommand::Params(params) => self.params = params,
            CompanionCommand::Tap(input) => {
                let _ = self.arrivals.send(Arrival::Tap(input));
            }
            CompanionCommand::Summon(input) => {
                let _ = self.arrivals.send(Arrival::Summon(input));
            }
            CompanionCommand::Dismiss => self.feed(CompanionEvent::Dismissed),
            CompanionCommand::LauncherClosed => self.feed(CompanionEvent::LauncherClosed),
            CompanionCommand::StopAll => self.stop_all(),
            CompanionCommand::Ask(input) => self.ask(input),
            CompanionCommand::Card { task, action } => self.card(task, action),
            CompanionCommand::Promote(task) => {
                if let Some(action) = self.state.answers.get(&task).and_then(promote_card) {
                    self.card(task, action);
                }
            }
            CompanionCommand::Cancel(task) => {
                self.on_bus("stopping an answer", move |connection| async move {
                    answer::cancel(&connection, &task).await
                });
            }
            CompanionCommand::Tell { agent, space, text } => {
                let at = self.unix();
                self.on_bus("telling an agent", move |connection| async move {
                    tell::tell(&connection, agent, space, text, at).await
                });
            }
            CompanionCommand::Run { run, control } => self.steer(run, control),
            CompanionCommand::Undo(scope) => self.undo(scope),
            CompanionCommand::Memory { .. } => {}
            CompanionCommand::Things { text, generation } => self.things(text, generation),
            CompanionCommand::Timeline { generation } => self.timeline(generation),
            CompanionCommand::RefreshJournal => self.refresh_journal(),
            CompanionCommand::Perform(call) => {
                self.on_bus("performing an action", move |connection| async move {
                    intents::perform(&connection, *call).await
                });
            }
        }
    }

    fn ask(&mut self, input: sill_model::companion::AskInput) {
        let Some(connection) = self.connection.clone() else {
            return;
        };
        let (inner, events, at) = (self.inner.clone(), self.events.clone(), self.unix());
        tokio::spawn(async move {
            match answer::ask(&connection, input, at, &events).await {
                Ok(path) => {
                    let _ = inner.send(Inner::Follow(path));
                }
                Err(why) => eprintln!("sill: companion: asking: {why}"),
            }
        });
    }

    /// Press `action` on `task`'s card when the answer offers that button and no press of it
    /// is going: the answer is busy at once, and the call answers on its own task.
    fn card(&mut self, task: TaskId, action: CardActionId) {
        let Some(connection) = self.connection.clone() else {
            return;
        };
        let offered = self.state.answers.get(&task).is_some_and(|answer| {
            matches!(answer.busy, Busy::Idle | Busy::Refused(_))
                && card_call(answer, &action).is_some()
        });
        if !offered {
            return;
        }
        self.feed(CompanionEvent::CardPressed {
            task: task.clone(),
            action: action.clone(),
        });
        tokio::spawn(answer::card(connection, task, action, self.events.clone()));
    }

    fn steer(&mut self, run: prov::RunId, control: RunControl) {
        match control {
            RunControl::UndoAll => self.undo(UndoScope::Run(run)),
            other => {
                self.on_bus("steering a run", move |connection| async move {
                    cua::control(&connection, &run, other).await
                });
            }
        }
    }

    fn stop_all(&mut self) {
        for run in going_runs(&self.state) {
            self.steer(run, RunControl::Pause);
        }
    }

    fn undo(&mut self, scope: UndoScope) {
        let events = self.events.clone();
        self.on_bus("undoing", move |connection| async move {
            let done = intents::undo(&connection, scope).await;
            if let Ok(rows) = intents::journal(&connection).await {
                let _ = events.send(CompanionEvent::Journal(rows));
            }
            done
        });
    }

    fn things(&mut self, text: String, generation: Generation) {
        self.feed(CompanionEvent::ThingsAsked(generation));
        match self.connection.clone() {
            Some(connection) => {
                tokio::spawn(intents::search(
                    connection,
                    text,
                    generation,
                    self.events.clone(),
                ));
            }
            None => self.feed(CompanionEvent::ThingsFailed(generation)),
        }
    }

    fn timeline(&mut self, generation: Generation) {
        self.feed(CompanionEvent::TimelineAsked(generation));
        match self.connection.clone() {
            Some(connection) => {
                let events = self.events.clone();
                tokio::spawn(async move {
                    let _ = events.send(memory::timeline(connection, generation).await);
                });
            }
            None => self.feed(CompanionEvent::TimelineFailed(generation)),
        }
    }

    fn refresh_journal(&mut self) {
        let events = self.events.clone();
        self.on_bus("reading the journal", move |connection| async move {
            let rows = intents::journal(&connection).await?;
            events.send(CompanionEvent::Journal(rows)).map_err(text)
        });
    }
}

/// Follow the companion's answer objects while it is on the bus.
async fn follow_answers(connection: Connection, inner: mpsc::UnboundedSender<Inner>) {
    loop {
        owner::appeared(&connection, COMPANION_BUS).await;
        tokio::select! {
            () = announce(&connection, &inner) => return,
            () = owner::changed(&connection, COMPANION_BUS) => {}
        }
    }
}

/// `AnswerAdded` and `AnswerRemoved` as they come; ends when the inner channel has no receiver.
async fn announce(connection: &Connection, inner: &mpsc::UnboundedSender<Inner>) {
    let Ok(proxy) = CompanionProxy::new(connection).await else {
        return std::future::pending().await;
    };
    let (Ok(mut added), Ok(mut removed)) = (
        proxy.receive_answer_added().await,
        proxy.receive_answer_removed().await,
    ) else {
        return std::future::pending().await;
    };
    loop {
        let sent = tokio::select! {
            Some(signal) = next(&mut added) => signal.args().ok().map(|args| {
                inner.send(Inner::Follow(OwnedObjectPath::from(args.answer().clone())))
            }),
            Some(signal) = next(&mut removed) => signal.args().ok().map(|args| {
                inner.send(Inner::Unfollow(OwnedObjectPath::from(args.answer().clone())))
            }),
            else => return,
        };
        if matches!(sent, Some(Err(_))) {
            return;
        }
    }
}

/// intentd's journal and late hits, while intentd is on the bus.
async fn follow_intentd(connection: Connection, events: mpsc::UnboundedSender<CompanionEvent>) {
    loop {
        owner::appeared(&connection, INTENTS_BUS).await;
        tokio::select! {
            () = intents::follow(connection.clone(), events.clone()) => return,
            () = owner::changed(&connection, INTENTS_BUS) => {}
        }
    }
}
