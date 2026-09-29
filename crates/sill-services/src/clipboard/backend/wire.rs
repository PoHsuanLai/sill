//! The clipboard's Wayland thread: its own connection (as `cosmic_wl` has; a protocol object
//! cannot cross connections), the data-control manager bound on the first seat, and a calloop
//! loop over the connection and the service's orders. Dropping the [`Wire`] ends the thread and
//! with it the connection.

use std::io;

use cosmic_client_toolkit::sctk::reexports::calloop;
use cosmic_client_toolkit::sctk::reexports::calloop::channel::Event;
use cosmic_client_toolkit::sctk::reexports::calloop::{EventLoop, channel};
use cosmic_client_toolkit::sctk::reexports::calloop_wayland_source::WaylandSource;
use cosmic_client_toolkit::wayland_client::globals::{GlobalError, registry_queue_init};
use cosmic_client_toolkit::wayland_client::protocol::wl_seat::WlSeat;
use sill_model::clipboard::{ClipContent, ClipEvent, ClipProtocol, ImageCap};
use tokio::sync::mpsc::UnboundedSender;

use super::wire_state::{Binding, WireState};
use super::wl::{bind_manager, manager_interfaces};
use crate::env::{WaylandRoute, WaylandUnreached};

/// Which connection an event came from: the service drops what an older one says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Epoch(pub(crate) u64);

/// An event from the thread of one connection.
#[derive(Debug)]
pub(crate) struct Heard {
    pub(crate) epoch: Epoch,
    pub(crate) event: ClipEvent,
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

    pub(crate) fn say(&self, event: ClipEvent) {
        // The service's task outlives every thread it starts; a closed channel means it is
        // ending too, and there is no one left to tell.
        let _ = self.to.send(Heard {
            epoch: self.epoch,
            event,
        });
    }
}

/// What the service tells the thread.
#[derive(Debug)]
pub(crate) enum Order {
    /// The image cap changed.
    Images(ImageCap),
    /// Make this the selection.
    Offer(ClipContent),
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
pub(crate) fn start(display: WaylandRoute, images: ImageCap, tell: Tell) -> io::Result<Wire> {
    let (orders, from_service) = channel::channel();
    std::thread::Builder::new()
        .name("sill-clipboard".to_owned())
        .spawn(move || run(&display, images, from_service, tell))?;
    Ok(Wire { orders })
}

/// Why the thread is not following a compositor (said by kind, never with content).
#[derive(Debug, thiserror::Error)]
enum Refusal {
    #[error("{0}")]
    Unreached(#[from] WaylandUnreached),
    #[error("the registry: {0}")]
    Registry(#[from] GlobalError),
    #[error("the event loop: {0}")]
    Loop(#[from] calloop::Error),
}

fn run(display: &WaylandRoute, images: ImageCap, orders: channel::Channel<Order>, tell: Tell) {
    if let Err(refusal) = follow(display, images, orders, tell.clone()) {
        eprintln!("sill: clipboard: {refusal}; history is not following the clipboard");
    }
    tell.say(ClipEvent::Disconnected);
}

/// Bind, report the protocol, and run the loop until the service drops its [`Wire`] or the
/// compositor goes away.
fn follow(
    display: &WaylandRoute,
    images: ImageCap,
    orders: channel::Channel<Order>,
    tell: Tell,
) -> Result<(), Refusal> {
    let conn = display.connect()?;
    let (globals, queue) = registry_queue_init::<WireState>(&conn)?;
    let qh = queue.handle();
    let seat = globals.bind::<WlSeat, _, _>(&qh, 1..=1, ()).ok();
    let binding = match (bind_manager(&globals, &qh), seat) {
        (Some(manager), Some(seat)) => {
            let device = manager.device(&seat, &qh);
            Binding::Bound { manager, device }
        }
        _ => {
            let [ext, wlr] = manager_interfaces();
            eprintln!("sill: clipboard: no seat, or neither {ext} nor {wlr}");
            Binding::Unsupported
        }
    };
    tell.say(ClipEvent::Connected(match &binding {
        Binding::Bound { manager, .. } => manager.protocol(),
        Binding::Unsupported => ClipProtocol::Unsupported,
    }));
    let mut event_loop = EventLoop::<WireState>::try_new()?;
    let handle = event_loop.handle();
    let signal = event_loop.get_signal();
    let mut state = WireState::new(
        conn.clone(),
        qh,
        handle.clone(),
        signal.clone(),
        binding,
        images,
        tell,
    );
    WaylandSource::new(conn, queue)
        .insert(handle.clone())
        .map_err(|error| error.error)?;
    handle
        .insert_source(orders, move |event, _, state: &mut WireState| match event {
            Event::Msg(Order::Images(cap)) => state.images = cap,
            Event::Msg(Order::Offer(content)) => state.offer_back(content),
            Event::Closed => signal.stop(),
        })
        .map_err(|error| error.error)?;
    event_loop.run(None, &mut state, |_| {})?;
    Ok(())
}
