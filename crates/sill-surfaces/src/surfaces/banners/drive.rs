//! The effects end of the column: follow the service's banners and the [`BannerLink`] into the
//! [`deck`](super::deck), carry out what it decides with quire's one timer (`ds::sleep`, for
//! durations that are quire's duration tokens or the hold from `notifications.*`, never a
//! literal), turn the pointer into the [`gesture`](super::gesture)'s inputs, and send the
//! service what a press means (`commands.rs`). Once the last card has gone, no task runs.
//!
//! [`BannerLink`]: super::BannerLink

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use dioxus::core::Task;
use dioxus::prelude::*;
use ds::{DurationToken, FRAME_SLACK, MotionLevel};
use shell_host::SurfaceHandle;
use sill_services::{
    ActionKey, Activation, ActivationToken, Control, LiveBanner, Notification, NotificationId,
    NotificationsCommand, Stamp,
};

use super::commands::action_command;
use super::deck::{Deck, DeckIn, DeckOut, DeckParams, EntryKey, advance};
use super::gesture::{self, Click, Press, Sample};
use super::machine::BannerIn;
use super::stack::Card;
use crate::FrameClock;
use crate::surfaces::notification_card::CardPress;

/// Everything the column's handlers share. Copy: signals and cells only.
#[derive(Clone, Copy)]
pub(crate) struct Column {
    pub deck: Signal<Deck>,
    press: CopyValue<Press>,
    params: CopyValue<DeckParams>,
    clock: FrameClock,
    control: CopyValue<Control<NotificationsCommand>>,
    surface: CopyValue<Option<SurfaceHandle>>,
    level: CopyValue<MotionLevel>,
    waking: CopyValue<Option<Task>>,
    /// The notifications the drawn cards show, kept while a card slides out after the service
    /// dropped it.
    notes: CopyValue<BTreeMap<NotificationId, Notification>>,
}

impl Column {
    /// Feed the deck `input` now and carry out what it says.
    pub fn input(self, input: DeckIn) {
        let now = self.clock.now();
        let (next, outs) = advance(self.deck.peek().clone(), input, now, &self.params.peek());
        let mut deck = self.deck;
        if *deck.peek() != next {
            deck.set(next);
        }
        for out in outs {
            self.perform(out);
        }
        self.rewake(now);
    }

    /// Keep `live`'s notifications, and only those and the ones a drawn card still shows.
    pub fn keep(self, live: &[LiveBanner]) {
        let mut notes = self.notes;
        let deck = self.deck.peek();
        let drawn: Vec<NotificationId> = deck
            .entries
            .iter()
            .flat_map(|e| e.card.grouped.iter().copied())
            .collect();
        let mut notes = notes.write();
        notes.retain(|id, _| drawn.contains(id));
        notes.extend(live.iter().map(|b| (b.note.id, b.note.clone())));
    }

    /// A kept notification.
    pub fn kept(&self, id: NotificationId) -> Option<Notification> {
        self.notes.peek().get(&id).cloned()
    }

    fn perform(self, out: DeckOut) {
        let level = *self.level.peek();
        match out {
            DeckOut::Rise(key) => self.after(FRAME_SLACK, DeckIn::Risen(key)),
            DeckOut::Entrance(key) => self.after(
                DurationToken::Big.duration(level) + FRAME_SLACK,
                DeckIn::Card(key, BannerIn::Entered),
            ),
            DeckOut::Exit(key) => self.after(
                DurationToken::Move.duration(level) + FRAME_SLACK,
                DeckIn::Card(key, BannerIn::Left),
            ),
            DeckOut::Send(command) => self.send(command),
        }
    }

    fn after(self, wait: Duration, input: DeckIn) {
        spawn(async move {
            ds::sleep(wait).await;
            self.input(input);
        });
    }

    /// Wake at the earliest hold's end (and only then).
    fn rewake(self, now: Stamp) {
        let mut waking = self.waking;
        if let Some(task) = waking.take() {
            task.cancel();
        }
        if let Some(at) = self.deck.peek().next_wake() {
            let wait = Duration::from_millis(at.since(now));
            waking.set(Some(spawn(async move {
                ds::sleep(wait).await;
                self.input(DeckIn::Tick);
            })));
        }
    }

    fn send(self, command: NotificationsCommand) {
        // A stopped service has nothing to tell: the banners are already gone with it.
        let _ = self.control.peek().send(command);
    }

    /// A button went down on `key`'s card at `x`.
    pub fn down(self, key: EntryKey, x: f64) {
        let at = self.sample(x);
        {
            let mut press = self.press;
            press.set(gesture::down(key, at));
        }
    }

    /// The pointer moved to `x` over the column.
    pub fn moved(self, x: f64) {
        let (next, inputs) = gesture::moved(*self.press.peek(), self.sample(x));
        {
            let mut press = self.press;
            press.set(next);
        }
        for (key, input) in inputs {
            self.input(DeckIn::Card(key, input));
        }
    }

    /// The button came up at `x`.
    pub fn up(self, x: f64) {
        let (next, inputs) = gesture::up(*self.press.peek(), self.sample(x));
        {
            let mut press = self.press;
            press.set(next);
        }
        for (key, input) in inputs {
            self.input(DeckIn::Card(key, input));
        }
    }

    fn sample(self, x: f64) -> Sample {
        Sample {
            x,
            at: self.clock.now(),
        }
    }

    /// `what` was pressed on `key`'s card, drawn as `note`.
    pub fn pressed(self, key: EntryKey, card: Card, note: &Notification, what: CardPress) {
        let (next, click) = gesture::click(*self.press.peek());
        {
            let mut press = self.press;
            press.set(next);
        }
        let input = match (what, click) {
            (CardPress::Body, Click::Swallowed) => return,
            (CardPress::Body, Click::Counts) => match note.default_action() {
                Some(_) => {
                    self.act(card, ActionKey::default_action());
                    BannerIn::Clicked
                }
                None => BannerIn::Close,
            },
            (CardPress::Action(action), _) => {
                self.act(card, action);
                BannerIn::Action
            }
            (CardPress::Close, _) => BannerIn::Close,
            (CardPress::Link(_), _) => return,
        };
        self.input(DeckIn::Card(key, input));
    }

    /// Invoke `action` on `card`'s newest notification, with an activation token for the app
    /// when the compositor gives one.
    fn act(self, card: Card, action: ActionKey) {
        let surface = self.surface.peek().clone();
        let app_id = Some(card.app.0.clone());
        spawn(async move {
            let token = match surface {
                Some(surface) => surface.activation_token(app_id).await,
                None => None,
            };
            let token = token.map_or(Activation::NoToken, |t| {
                Activation::Token(ActivationToken(t))
            });
            if let Some(command) = action_command(&card, action, token) {
                self.send(command);
            }
        });
    }
}

/// The column's shared state, the motion `level` read inside its `Ds`.
pub(crate) fn use_column(
    params: DeckParams,
    control: Control<NotificationsCommand>,
    level: MotionLevel,
) -> Column {
    let surface = try_use_context::<SurfaceHandle>();
    let column = use_hook(|| Column {
        deck: Signal::new(Deck::default()),
        press: CopyValue::new(Press::default()),
        params: CopyValue::new(params),
        clock: FrameClock::new(Instant::now()),
        control: CopyValue::new(control),
        surface: CopyValue::new(surface),
        level: CopyValue::new(level),
        waking: CopyValue::new(None),
        notes: CopyValue::new(BTreeMap::new()),
    });
    if *column.params.peek() != params {
        let mut cell = column.params;
        cell.set(params);
    }
    if *column.level.peek() != level {
        let mut cell = column.level;
        cell.set(level);
    }
    column
}
