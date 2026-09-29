//! The effects end of the column: follow the service's banners and the [`BannerLink`] into the
//! [`deck`](super::deck), carry out what it decides with quire's one timer (`ds::sleep`, for
//! quire's `FRAME_SLACK` or the hold from `notifications.*`, never a literal), turn the card's
//! presses, hover and swipe and the stack's `on_hidden` into the deck's inputs, and send the
//! service what a press means (`commands.rs`). Once the last card has gone, no task runs.
//!
//! [`BannerLink`]: crate::links::BannerLink

use std::collections::BTreeMap;
use std::time::Duration;

use dioxus::core::Task;
use dioxus::prelude::*;
use ds::{FRAME_SLACK, Hover};
use shell_host::SurfaceHandle;
use sill_model::notifications::{
    ActionKey, Activation, ActivationToken, LiveBanner, Notification, NotificationId,
    NotificationsCommand,
};
use sill_model::units::Stamp;
use sill_services::Control;

use super::commands::action_command;
use super::deck::{Deck, DeckIn, DeckOut, EntryKey, advance};
use super::machine::{BannerIn, BannerParams};
use super::stack::Card;
use crate::clock::FrameClock;
use crate::notification_card::CardPress;

/// Everything the column's handlers share. Copy: signals and cells only.
#[derive(Clone, Copy)]
pub(crate) struct Column {
    pub deck: Signal<Deck>,
    params: CopyValue<BannerParams>,
    clock: FrameClock,
    control: CopyValue<Control<NotificationsCommand>>,
    surface: CopyValue<Option<SurfaceHandle>>,
    waking: CopyValue<Option<Task>>,
    /// The notifications the listed cards show, kept a moment past the service's own drop.
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
        match out {
            DeckOut::Rise(key) => {
                spawn(async move {
                    ds::sleep(FRAME_SLACK).await;
                    self.input(DeckIn::Risen(key));
                });
            }
            DeckOut::Send(command) => self.send(command),
        }
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

    /// The pointer came to or left `key`'s card: its hold pauses or resumes.
    pub fn hovered(self, key: EntryKey, hover: Hover) {
        let input = match hover {
            Hover::Over => BannerIn::HoverIn,
            Hover::Away => BannerIn::HoverOut,
        };
        self.input(DeckIn::Card(key, input));
    }

    /// quire's stack has played `key`'s card out.
    pub fn hidden(self, key: EntryKey) {
        self.input(DeckIn::Card(key, BannerIn::Left));
    }

    /// `what` was pressed on `key`'s card, drawn as `note`.
    pub fn pressed(self, key: EntryKey, card: Card, note: &Notification, what: CardPress) {
        let input = match what {
            CardPress::Body => match note.default_action() {
                Some(_) => {
                    self.act(card, ActionKey::default_action());
                    BannerIn::Clicked
                }
                None => BannerIn::Close,
            },
            CardPress::Action(action) => {
                self.act(card, action);
                BannerIn::Action
            }
            CardPress::Close => BannerIn::Close,
            CardPress::Link(href) => {
                // sill has no URL opener yet: the link keeps its press.
                eprintln!("banners.link={}", href.0);
                return;
            }
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

/// The column's shared state.
pub(crate) fn use_column(params: BannerParams, control: Control<NotificationsCommand>) -> Column {
    let surface = try_use_context::<SurfaceHandle>();
    let column = use_hook(|| Column {
        deck: Signal::new(Deck::default()),
        params: CopyValue::new(params),
        clock: FrameClock::started(),
        control: CopyValue::new(control),
        surface: CopyValue::new(surface),
        waking: CopyValue::new(None),
        notes: CopyValue::new(BTreeMap::new()),
    });
    if *column.params.peek() != params {
        let mut cell = column.params;
        cell.set(params);
    }
    column
}
