//! The DDC/CI wire the displays service reads and writes: [`DdcBus`], the kernel's i2c buses
//! ([`I2cBuses`]), made-up monitors in a file ([`FileBus`]) and, in tests, maps ([`FakeBus`]).

mod bus;
#[cfg(test)]
pub(crate) mod fake;
mod file_bus;
mod i2c;

pub use bus::{DdcBus, DdcError, DdcId, Endpoint};
#[cfg(test)]
pub(crate) use fake::{FakeBus, FakeState};
pub use file_bus::FileBus;
pub use i2c::I2cBuses;
