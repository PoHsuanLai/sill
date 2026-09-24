//! `use_services`: the root context every other hook reads.

use dioxus::prelude::use_context;
use sill_services::Services;

/// The daemon's services, provided once per surface with `Root::context`.
///
/// # Panics
/// When the surface's `Root` was built without `.context(services)`: a daemon wiring bug,
/// Never a runtime condition.
pub fn use_services() -> Services {
    use_context::<Services>()
}
