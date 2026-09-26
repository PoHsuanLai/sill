//! Async bus calls from the provider's synchronous `query`. The runner calls every provider on
//! the blocking pool (`spawn_blocking`), where `Handle::block_on` is the documented way back
//! into async code (F107); with no runtime at all (an example, a plain test) a current-thread
//! runtime is made for the one call.

use std::future::Future;

use tokio::runtime::{Builder, Handle};

/// `future`'s output, or `None` when no runtime could run it.
pub(super) fn block_on<F: Future>(future: F) -> Option<F::Output> {
    match Handle::try_current() {
        Ok(handle) => Some(handle.block_on(future)),
        Err(_) => Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
            .map(|runtime| runtime.block_on(future)),
    }
}
