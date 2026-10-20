//! The bar clock: a task that sleeps to each minute boundary and publishes the local time.

mod backend;

use std::sync::Arc;

use sill_model::clock::{ClockCommand, ClockMinute};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The bar clock as a [`Service`]: publishes the wall clock's local time now, then at every
/// minute boundary (the system clock reads the zone afresh, so a time-zone change shows at the
/// next minute, or at once on `Refresh`).
#[derive(Debug)]
pub enum ClockService {}

impl Service for ClockService {
    const NAME: &'static str = "clock";
    type State = ClockMinute;
    type Command = ClockCommand;
    type Deps = ();

    fn initial(env: &Env, (): &()) -> ClockMinute {
        ClockMinute(env.clock.now().datetime())
    }

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let clock = Arc::clone(&env.clock);
        async move {
            backend::run(io.state, io.commands, clock).await;
            Ok(())
        }
    }
}
