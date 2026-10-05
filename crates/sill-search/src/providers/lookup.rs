//! Waiting for the companion service's answer to a lookup from a provider's synchronous `query`.
//! The runner calls every provider on the blocking pool, where `Handle::block_on` is the way
//! back into async code; with no runtime (a plain test) a current-thread runtime is made for the
//! one wait.

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
