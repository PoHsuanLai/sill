//! A service's own Wayland connection on a thread of its own (a protocol object cannot cross
//! connections): connect to the compositor, bind what the service needs, and run a calloop loop
//! over the connection and the service's orders until the service drops its [`Wire`] or the
//! compositor goes away. What the thread saw goes back to the service's task as events tagged
//! with the connection they came from, so the service drops what an older one says.
//!
//! A service supplies a state type implementing [`Follow`]: how it binds its globals, what it
//! does with an order, and the event that says the thread has ended. The rest is here once.

use std::io;

use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::reexports::calloop::channel::Event;
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopHandle, LoopSignal, channel};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use tokio::sync::mpsc::UnboundedSender;
use wayland_client::globals::{GlobalError, GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::env::{WaylandRoute, WaylandUnreached};

/// Which connection an event came from: the service drops what an older one says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Epoch(pub(crate) u64);

/// An event, tagged with its connection.
#[derive(Debug)]
pub(crate) struct Heard<V> {
    pub(crate) epoch: Epoch,
    pub(crate) event: V,
}

/// How the thread reports to the service's task.
#[derive(Debug)]
pub(crate) struct Tell<V> {
    epoch: Epoch,
    to: UnboundedSender<Heard<V>>,
}

impl<V> Clone for Tell<V> {
    fn clone(&self) -> Self {
        Tell {
            epoch: self.epoch,
            to: self.to.clone(),
        }
    }
}

impl<V> Tell<V> {
    pub(crate) fn new(epoch: Epoch, to: UnboundedSender<Heard<V>>) -> Tell<V> {
        Tell { epoch, to }
    }

    pub(crate) fn say(&self, event: V) {
        // The service's task outlives every thread it starts; a closed channel means it is
        // ending too, and there is no one left to tell.
        let _ = self.to.send(Heard {
            epoch: self.epoch,
            event,
        });
    }
}

/// A running connection; dropping it ends the thread.
#[derive(Debug)]
pub(crate) struct Wire<O> {
    orders: channel::Sender<O>,
}

impl<O> Wire<O> {
    pub(crate) fn order(&self, order: O) {
        // The thread has ended (the compositor went away); it said so, and orders are moot.
        let _ = self.orders.send(order);
    }
}

/// What a state is built from, once the connection and the registry are there.
pub(crate) struct Setup<F: Follow> {
    pub(crate) conn: Connection,
    pub(crate) globals: GlobalList,
    pub(crate) qh: QueueHandle<F>,
    pub(crate) handle: LoopHandle<'static, F>,
    pub(crate) signal: LoopSignal,
    pub(crate) tell: Tell<F::Event>,
}

/// The state of one service's Wayland thread.
pub(crate) trait Follow: Dispatch<WlRegistry, GlobalListContents> + Sized + 'static {
    /// What the service gives the thread when it starts it.
    type Start: Send + 'static;
    /// What the service tells the thread.
    type Order: Send + 'static;
    /// What the thread tells the service.
    type Event: Send + 'static;

    /// The service's name: the thread is `sill-<NAME>`, and its lines start `sill: <NAME>:`.
    const NAME: &'static str;
    /// What the line says follows when the thread is not following a compositor.
    const UNFOLLOWED: &'static str;

    /// Bind the globals the service needs, and report what was found.
    fn bind(start: Self::Start, setup: Setup<Self>) -> Self;

    /// Carry out an order from the service.
    fn order(&mut self, order: Self::Order);

    /// The event that says the thread has ended.
    fn gone() -> Self::Event;
}

/// Start a thread connected to `display`.
pub(crate) fn start<F: Follow>(
    display: WaylandRoute,
    start: F::Start,
    tell: Tell<F::Event>,
) -> io::Result<Wire<F::Order>> {
    let (orders, from_service) = channel::channel();
    std::thread::Builder::new()
        .name(format!("sill-{}", F::NAME))
        .spawn(move || run::<F>(&display, start, from_service, tell))?;
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

fn run<F: Follow>(
    display: &WaylandRoute,
    start: F::Start,
    orders: channel::Channel<F::Order>,
    tell: Tell<F::Event>,
) {
    if let Err(refusal) = follow::<F>(display, start, orders, tell.clone()) {
        eprintln!("sill: {}: {refusal}; {}", F::NAME, F::UNFOLLOWED);
    }
    tell.say(F::gone());
}

/// Bind, report, and run the loop until the service drops its [`Wire`] or the compositor goes.
fn follow<F: Follow>(
    display: &WaylandRoute,
    start: F::Start,
    orders: channel::Channel<F::Order>,
    tell: Tell<F::Event>,
) -> Result<(), Refusal> {
    let conn = display.connect()?;
    let (globals, queue) = registry_queue_init::<F>(&conn)?;
    let mut event_loop = EventLoop::<F>::try_new()?;
    let handle = event_loop.handle();
    let signal = event_loop.get_signal();
    let mut state = F::bind(
        start,
        Setup {
            conn: conn.clone(),
            globals,
            qh: queue.handle(),
            handle: handle.clone(),
            signal: signal.clone(),
            tell,
        },
    );
    WaylandSource::new(conn, queue)
        .insert(handle.clone())
        .map_err(|error| error.error)?;
    handle
        .insert_source(orders, move |event, _, state: &mut F| match event {
            Event::Msg(order) => state.order(order),
            Event::Closed => signal.stop(),
        })
        .map_err(|error| error.error)?;
    event_loop.run(None, &mut state, |_| {})?;
    Ok(())
}
