//! The wake-up for events another thread's read queued for the arm.
//!
//! The arm's thread is not the only reader of the host's connection: the host's own thread and
//! Mesa's reads for a GPU surface read the socket too, and such a read moves the arm's events into
//! the arm's queue at once. The socket's descriptor is then quiet, and libwayland's queue is not
//! something a loop can wait on, so the events would sit there. The client leaves a waker in its
//! queue; the queue wakes it when any thread's read adds an event, and its ping dispatches the
//! queue on the arm's thread.

use std::io;
use std::sync::Arc;
use std::task::{Wake, Waker};

use smithay_client_toolkit::reexports::calloop::ping::{Ping, PingSource, make_ping};

/// Pings the loop when a read on another thread queues an event for the arm.
struct QueueWaker(Ping);

impl Wake for QueueWaker {
    fn wake(self: Arc<Self>) {
        self.0.ping();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.ping();
    }
}

/// The waker the client's queue holds, and the loop source its ping arrives on.
pub(super) fn waker_and_source() -> io::Result<(Waker, PingSource)> {
    let (ping, source) = make_ping()?;
    Ok((Waker::from(Arc::new(QueueWaker(ping))), source))
}
