//! Idle's Wayland thread: its own connection to [`WaylandRoute`] (a protocol object cannot cross
//! connections), `ext_idle_notifier_v1` for the watches (`get_idle_notification`, which honours
//! the compositor's idle inhibitors; never the input form), and `zwlr_output_power_manager_v1` with
//! one `zwlr_output_power_v1` per `wl_output` for the displays. A calloop loop runs over the
//! connection and the service's [`Order`]s. Dropping the [`Wire`] ends the thread and with it the
//! connection.

use std::io;

use sill_model::idle::{DisplayPower, Threshold, Watch};

use super::wl_state::WlState;
use crate::env::WaylandRoute;
use crate::wayland_thread::{self as thread, Follow, Setup};

pub(crate) use crate::wayland_thread::Epoch;

/// An event, tagged with its connection.
pub(crate) type Heard = thread::Heard<WlEvent>;

/// How the thread reports to the service's task.
pub(crate) type Tell = thread::Tell<WlEvent>;

/// A running connection; dropping it ends the thread.
pub(crate) type Wire = thread::Wire<Order>;

/// What the thread saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WlEvent {
    /// Connected; whether the two protocols are there.
    Bound(Protocols),
    /// `idled` on the watch of that threshold.
    Idled(Threshold),
    /// `resumed`: real input after an `idled`.
    Resumed,
    /// The thread ended (no compositor, or it went away).
    Gone,
}

/// Which of idle's protocols the compositor offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Protocols {
    pub(crate) notifier: Offered,
    pub(crate) output_power: Offered,
}

/// Whether a global is offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Offered {
    Yes,
    No,
}

/// What the service tells the thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Order {
    /// Hold these notifications: an unchanged watch is kept (its timer runs on); one that idled is
    /// kept until it resumes, so the wake is never lost.
    Watch(Vec<Watch>),
    /// Drop every notification, idled or not, and hold these afresh (after a made-up wake, which
    /// the compositor never saw).
    Renew(Vec<Watch>),
    Displays(DisplayPower),
}

/// Start a thread connected to `display`.
pub(crate) fn start(display: WaylandRoute, tell: Tell) -> io::Result<Wire> {
    thread::start::<WlState>(display, (), tell)
}

impl Follow for WlState {
    type Start = ();
    type Order = Order;
    type Event = WlEvent;

    const NAME: &'static str = "idle";
    const UNFOLLOWED: &'static str = "idle is not followed";

    fn bind((): (), setup: Setup<WlState>) -> WlState {
        let state = WlState::bind(&setup.globals, setup.qh, setup.tell.clone());
        setup.tell.say(WlEvent::Bound(state.protocols()));
        state
    }

    fn order(&mut self, order: Order) {
        match order {
            Order::Watch(watches) => self.watch(watches),
            Order::Renew(watches) => self.renew(watches),
            Order::Displays(power) => self.displays(power),
        }
    }

    fn gone() -> WlEvent {
        WlEvent::Gone
    }
}
