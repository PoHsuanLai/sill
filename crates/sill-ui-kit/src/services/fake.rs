//! What a fake [`Services`](super::Services) hands the harness: each service's halves of its
//! channels, which a test claims or leaves to be kept alive.

use sill_services::{Service, ServiceIo};

/// One service's halves of its channels, until something claims them.
pub(crate) struct Claim<S: Service>(Option<ServiceIo<S>>);

impl<S: Service> Claim<S> {
    pub(crate) fn new(io: ServiceIo<S>) -> Claim<S> {
        Claim(Some(io))
    }

    /// The halves, for the harness to expose; claiming a service twice is a bug in the harness.
    pub(crate) fn take(&mut self) -> ServiceIo<S> {
        self.0
            .take()
            .unwrap_or_else(|| panic!("the fake {} was claimed twice", S::NAME))
    }

    /// Keep the halves alive, if nothing claimed them.
    pub(crate) fn keep(self, keep: &mut Vec<Box<dyn std::any::Any>>) {
        if let Some(io) = self.0 {
            keep.push(Box::new(io));
        }
    }
}
