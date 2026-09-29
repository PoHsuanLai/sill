//! The battery: UPower's display device over the system bus.

mod live;
mod model;

pub use model::{Battery, BatteryEvent, Charge, Remaining, step};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// What the battery service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryCommand {
    /// Re-read every property (after a resume).
    Refresh,
}

/// UPower's display device as a [`Service`]. Without UPower the state stays at
/// `Battery::default()` (absent), and the bar hides the item.
#[derive(Debug)]
pub enum BatteryService {}

impl Service for BatteryService {
    const NAME: &'static str = "battery";
    type State = Battery;
    type Command = BatteryCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        live::run(env.buses.system.clone(), io.state, io.commands)
    }
}
