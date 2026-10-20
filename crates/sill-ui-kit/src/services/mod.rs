//! The one value every surface gets through `Root::context`: every service's [`Handle`].
//! Clone is cheap (channels are reference-counted). The daemon starts them
//! (`Services::start`); `registry` is the one place a service is declared.

#[macro_use]
mod declare;
#[cfg(feature = "harness")]
mod fake;
mod registry;
mod wiring;

#[cfg(feature = "harness")]
pub(crate) use registry::Ios;
pub use registry::Services;
use sill_services::service::{Handle, Service};
pub use wiring::Wiring;

/// A service [`Services`] holds, and which field holds it: what lets a reader name a service by
/// its type alone (`sill-ui-kit`' `use_service::<S>()`).
pub trait Member: Service {
    fn of(services: &Services) -> &Handle<Self>;
}

#[cfg(all(test, feature = "harness"))]
mod tests;
