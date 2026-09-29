//! The seam between the DDC worker and the wire: [`DdcBus`], implemented over `/dev/i2c-N`
//! ([`super::I2cBuses`]) and by a fake in the tests.

use sill_model::displays::ddc::Endpoint;
use sill_model::displays::ddc::vcp::{Feature, Level};
use sill_model::ids::DdcId;

/// Why one DDC transaction did not happen. Never fatal: the capability degrades.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DdcError {
    /// The bus is gone or refused us (unplugged, no permission on `/dev/i2c-N`).
    #[error("i2c: {0}")]
    Bus(String),
    /// The display answered with something that is not a reply, or answered "unsupported".
    #[error("ddc: {0}")]
    Protocol(String),
    /// The worker has no display by that id.
    #[error("no DDC display {0:?}")]
    Unknown(DdcId),
}

/// Every DDC transaction the worker makes: the kernel's buses ([`super::I2cBuses`]) or made-up
/// monitors in a file ([`super::FileBus`], the dev scripts'). Blocking: each call is a
/// bus transaction of tens of milliseconds, and only the worker thread calls them (one at a
/// time, which is also what the bus needs).
pub trait DdcBus: Send + Sync + std::fmt::Debug {
    /// The displays that answer now. Re-probing replaces what an earlier probe found.
    fn probe(&self) -> Vec<Endpoint>;
    fn read(&self, id: DdcId, feature: Feature) -> Result<Level, DdcError>;
    fn write(&self, id: DdcId, feature: Feature, value: u16) -> Result<(), DdcError>;
}
