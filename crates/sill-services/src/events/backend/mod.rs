//! The service's effects: what it reaches outside the process.

mod live;
mod live_watch;
mod provider;
mod read;

pub(crate) use live::{Reach, run};
pub use provider::{IcsProvider, ProviderRead, provider_for};
