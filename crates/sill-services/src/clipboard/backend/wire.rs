//! The clipboard's Wayland thread ([`crate::wayland_thread`]): the data-control manager bound on
//! the first seat, and the orders the service sends. Dropping the [`Wire`] ends the thread and
//! with it the connection.

use std::io;

use sill_model::clipboard::{ClipContent, ClipEvent, ClipProtocol, ImageCap};
use wayland_client::protocol::wl_seat::WlSeat;

use super::wire_state::{Binding, WireState};
use super::wl::{bind_manager, manager_interfaces};
use crate::env::WaylandRoute;
use crate::wayland_thread::{self as thread, Follow, Setup};

pub(crate) use crate::wayland_thread::Epoch;

/// An event from the thread of one connection.
pub(crate) type Heard = thread::Heard<ClipEvent>;

/// How the thread reports to the service's task.
pub(crate) type Tell = thread::Tell<ClipEvent>;

/// A running connection; dropping it ends the thread.
pub(crate) type Wire = thread::Wire<Order>;

/// What the service tells the thread.
#[derive(Debug)]
pub(crate) enum Order {
    /// The image cap changed.
    Images(ImageCap),
    /// Make this the selection.
    Offer(ClipContent),
}

/// Start a thread connected to `display`.
pub(crate) fn start(display: WaylandRoute, images: ImageCap, tell: Tell) -> io::Result<Wire> {
    thread::start::<WireState>(display, images, tell)
}

impl Follow for WireState {
    type Start = ImageCap;
    type Order = Order;
    type Event = ClipEvent;

    const NAME: &'static str = "clipboard";
    const UNFOLLOWED: &'static str = "history is not following the clipboard";

    /// Bind the data-control manager on the first seat, and report the protocol.
    fn bind(images: ImageCap, setup: Setup<WireState>) -> WireState {
        let Setup {
            conn,
            globals,
            qh,
            handle,
            signal,
            tell,
        } = setup;
        let seat = globals.bind::<WlSeat, _, _>(&qh, 1..=1, ()).ok();
        let binding = if let (Some(manager), Some(seat)) = (bind_manager(&globals, &qh), seat) {
            let device = manager.device(&seat, &qh);
            Binding::Bound { manager, device }
        } else {
            let [ext, wlr] = manager_interfaces();
            eprintln!("sill: clipboard: no seat, or neither {ext} nor {wlr}");
            Binding::Unsupported
        };
        tell.say(ClipEvent::Connected(match &binding {
            Binding::Bound { manager, .. } => manager.protocol(),
            Binding::Unsupported => ClipProtocol::Unsupported,
        }));
        WireState::new(conn, qh, handle, signal, binding, images, tell)
    }

    fn order(&mut self, order: Order) {
        match order {
            Order::Images(cap) => self.images = cap,
            Order::Offer(content) => self.offer_back(content),
        }
    }

    fn gone() -> ClipEvent {
        ClipEvent::Disconnected
    }
}
