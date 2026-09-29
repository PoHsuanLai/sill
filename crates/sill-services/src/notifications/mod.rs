//! Notifications (design/20 §1.6, design/13 §13.3.6): an `org.freedesktop.Notifications` server
//! on the session bus, the live banners, and the history grouped by app, kept in
//! `$XDG_STATE_HOME/sill/notifications.json`.
//!
//! **It never fights another daemon.** The service asks for the name only while the daemon's
//! policy says `notifications.server = "own"` (the setting's default), with `AllowReplacement |
//! DoNotQueue` and never `ReplaceExisting`: when cosmic-notifications, mako or plasmashell holds
//! it, sill stays passive, reports the holder ([`NameHolder::Other`]), and asks again only when
//! the name falls free. Until the daemon's first [`NotificationsCommand::Policy`] the service is
//! passive, so a test or a headless render that spawns [`crate::Services`] never takes the name.
//!
//! **Where the time goes.** The banner's hold (`notifications.banner_hold_ms`, paused while
//! hovered) is counted by the banner surface's machine, which says so with
//! [`NotificationsCommand::BannerTimedOut`]; the sender's `expire_timeout` only decides whether
//! a banner persists (0). A banner leaving by itself keeps its notification in the history.
//!
//! **Without D-Bus.** `sill debug notify` posts a [`Posting`] straight to the service
//! ([`NotificationsCommand::Post`]), through the same reducer, so the banners and the center can
//! be accepted without a bus (and on a machine whose session already has a notification daemon).
//!
//! The shape is every service's: [`spawn`] returns the receivers and a control, and
//! [`reduce_notifications`] is the only place the state changes.

mod capabilities;
mod content;
mod dbus;
pub mod history_file;
mod ids;
mod live;
mod markup;
mod markup_parse;
mod name_watch;
mod policy;
mod reduce;
mod rules;
mod state;
mod wire;

pub use capabilities::{CAPABILITIES, SERVER_INFORMATION, SPEC_VERSION, ServerInformation};
pub use content::{
    Action, AppRef, Expiry, Hints, IconRef, NoteImage, Notification, NotifyUrgency, Pixels,
    Posting, Residency, SoundHint, Transience, pair_actions,
};
pub use dbus::{BUS_NAME, OBJECT_PATH};
pub use history_file::StoredHistory;
pub use ids::{ActionKey, Activation, ActivationToken, AppKey, NotificationId, UnixMs};
pub use markup::{Body, BodyRun, Href, RunLink, RunStyle, Slant, Underline, Weight, parse_body};
pub use policy::{
    Chime, HistoryCap, NotificationsPolicy, Presentation, Presented, Quiet, Swipe, chime, presented,
};
pub use reduce::{
    Clear, CloseReason, NotesState, NotificationsEffect, NotificationsEvent, assign_id,
    reduce_notifications,
};
pub use state::{
    AppGroup, BannerLife, History, LiveBanner, NameHolder, NameOwner, Notifications, ServerMode,
    ServerStatus,
};

use std::path::PathBuf;

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// What the notifications service can be told: by the daemon (the policy, a debug post), by the
/// banners and the center (the person's actions), and by `sill notifications`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationsCommand {
    /// The settings and focus mode now in effect; the first one ends the passive start.
    Policy(NotificationsPolicy),
    /// A notification as if `Notify` had been called (`sill debug notify`); no reply id.
    Post(Box<Posting>),
    /// The person dismissed it (the close button, a history row's clear).
    Dismiss(NotificationId),
    /// The person swiped or dragged its banner away: kept in the center or dismissed, as
    /// `notifications.swipe` says ([`Swipe`]).
    SwipeAway(NotificationId),
    /// Dismiss every live banner.
    DismissAll,
    /// A banner's hold ran out.
    BannerTimedOut(NotificationId),
    /// The person chose `action` (`default`: the body was clicked).
    InvokeAction {
        id: NotificationId,
        action: ActionKey,
        token: Activation,
    },
    /// The center's Clear.
    ClearHistory(Clear),
}

/// Where the history is kept.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HistoryPlace {
    /// `$XDG_STATE_HOME/sill/notifications.json` ([`history_file::default_path`]).
    #[default]
    State,
    /// A file of the caller's (tests).
    At(PathBuf),
    /// Nowhere: nothing read, nothing written.
    NotKept,
}

/// How the service is started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    /// The session bus the server speaks on (a private `dbus-daemon` in every test).
    pub bus: crate::env::Bus,
    pub history: HistoryPlace,
}

impl Setup {
    /// On `env`'s session bus, the history in `$XDG_STATE_HOME`.
    pub fn of(env: &crate::env::Env) -> Setup {
        Setup {
            bus: env.buses.session.clone(),
            history: HistoryPlace::State,
        }
    }
}

/// The service's two receivers and its control.
pub type NotificationsHandles = (
    watch::Receiver<Notifications>,
    watch::Receiver<History>,
    Control<NotificationsCommand>,
);

/// Start the service on `rt` as `setup` says. Passive until the daemon's first policy (module
/// doc).
pub fn spawn(rt: &Handle, setup: Setup) -> NotificationsHandles {
    let (live_tx, live) = watch::channel(Notifications::default());
    let (history_tx, history) = watch::channel(History::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(setup, live_tx, history_tx, commands));
    (live, history, control)
}
