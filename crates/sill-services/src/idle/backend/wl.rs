//! Idle's Wayland thread: its own connection to [`WaylandRoute`] (a protocol object cannot cross
//! connections), `ext_idle_notifier_v1` for the watches (`get_idle_notification`, which honours
//! the compositor's idle inhibitors; never the input form), and `zwlr_output_power_manager_v1` with
//! one `zwlr_output_power_v1` per `wl_output` for the displays. A calloop loop runs over the
//! connection and the service's [`Order`]s. Dropping the [`Wire`] ends the thread and with it the
//! connection.

use std::io;

use cosmic_client_toolkit::sctk::reexports::calloop;
use cosmic_client_toolkit::sctk::reexports::calloop::channel::Event;
use cosmic_client_toolkit::sctk::reexports::calloop::{EventLoop, channel};
use cosmic_client_toolkit::sctk::reexports::calloop_wayland_source::WaylandSource;
use cosmic_client_toolkit::wayland_client::globals::{GlobalError, registry_queue_init};
use sill_model::idle::{DisplayPower, Threshold, Watch};
use tokio::sync::mpsc::UnboundedSender;

use super::wl_state::WlState;
use crate::env::{WaylandRoute, WaylandUnreached};

/// Which connection an event came from: the service drops what an older one says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Epoch(pub(crate) u64);

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

/// An event, tagged with its connection.
#[derive(Debug)]
pub(crate) struct Heard {
    pub(crate) epoch: Epoch,
    pub(crate) event: WlEvent,
}

/// How the thread reports to the service's task.
#[derive(Debug, Clone)]
pub(crate) struct Tell {
    epoch: Epoch,
    to: UnboundedSender<Heard>,
}

impl Tell {
    pub(crate) fn new(epoch: Epoch, to: UnboundedSender<Heard>) -> Tell {
        Tell { epoch, to }
    }

    pub(crate) fn say(&self, event: WlEvent) {
        // The service outlives every thread it starts; a closed channel means it is ending.
        let _ = self.to.send(Heard {
            epoch: self.epoch,
            event,
        });
    }
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

/// A running connection; dropping it ends the thread.
#[derive(Debug)]
pub(crate) struct Wire {
    orders: channel::Sender<Order>,
}

impl Wire {
    pub(crate) fn order(&self, order: Order) {
        // The thread has ended (the compositor went away); it said so, and orders are moot.
        let _ = self.orders.send(order);
    }
}

/// Start a thread connected to `display`.
pub(crate) fn start(display: WaylandRoute, tell: Tell) -> io::Result<Wire> {
    let (orders, from_service) = channel::channel();
    std::thread::Builder::new()
        .name("sill-idle".to_owned())
        .spawn(move || run(&display, from_service, tell))?;
    Ok(Wire { orders })
}

/// Why the thread is not following a compositor.
#[derive(Debug, thiserror::Error)]
enum Refusal {
    #[error("{0}")]
    Unreached(#[from] WaylandUnreached),
    #[error("the registry: {0}")]
    Registry(#[from] GlobalError),
    #[error("the event loop: {0}")]
    Loop(#[from] calloop::Error),
}

fn run(display: &WaylandRoute, orders: channel::Channel<Order>, tell: Tell) {
    if let Err(refusal) = follow(display, orders, tell.clone()) {
        eprintln!("sill: idle: {refusal}; idle is not followed");
    }
    tell.say(WlEvent::Gone);
}

/// Bind, report, and run the loop until the service drops its [`Wire`] or the compositor goes.
fn follow(
    display: &WaylandRoute,
    orders: channel::Channel<Order>,
    tell: Tell,
) -> Result<(), Refusal> {
    let conn = display.connect()?;
    let (globals, queue) = registry_queue_init::<WlState>(&conn)?;
    let qh = queue.handle();
    let mut state = WlState::bind(&globals, qh, tell.clone());
    tell.say(WlEvent::Bound(state.protocols()));
    let mut event_loop = EventLoop::<WlState>::try_new()?;
    let handle = event_loop.handle();
    let signal = event_loop.get_signal();
    WaylandSource::new(conn, queue)
        .insert(handle.clone())
        .map_err(|error| error.error)?;
    handle
        .insert_source(orders, move |event, _, state: &mut WlState| match event {
            Event::Msg(Order::Watch(watches)) => state.watch(watches),
            Event::Msg(Order::Renew(watches)) => state.renew(watches),
            Event::Msg(Order::Displays(power)) => state.displays(power),
            Event::Closed => signal.stop(),
        })
        .map_err(|error| error.error)?;
    event_loop.run(None, &mut state, |_| {})?;
    Ok(())
}
