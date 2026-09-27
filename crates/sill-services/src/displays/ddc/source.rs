//! Which DDC buses the worker talks to: the kernel's (`/dev/i2c-N`), or, in a build with the
//! `fake-ddc` feature (sill's `debug` build, for the acceptance scripts), a file of made-up
//! monitors named by `SILL_FAKE_DDC` (`super::file_bus`), so a script can drive the brightness
//! path end to end without ever reaching a real monitor.

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use tokio::sync::watch;

use super::i2c::I2cBuses;
use super::worker::{self, DdcDisplay, DdcJob};

/// The DDC buses the displays service uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DdcSource {
    /// The kernel's i2c buses.
    Buses,
    /// The made-up monitors in this file (served only by a `fake-ddc` build).
    Fake(PathBuf),
}

impl DdcSource {
    /// `SILL_FAKE_DDC`'s file in a `fake-ddc` build when it is set; the real buses otherwise.
    pub fn from_env() -> DdcSource {
        #[cfg(feature = "fake-ddc")]
        if let Some(path) = std::env::var_os("SILL_FAKE_DDC") {
            let path = PathBuf::from(path);
            eprintln!(
                "sill: displays: DDC is the fake in {} (SILL_FAKE_DDC): no real monitor is touched",
                path.display()
            );
            return DdcSource::Fake(path);
        }
        DdcSource::Buses
    }

    /// Run the DDC worker on these buses until `jobs` closes. Blocking.
    pub fn serve(self, jobs: Receiver<DdcJob>, tx: watch::Sender<Vec<DdcDisplay>>) {
        match self {
            DdcSource::Buses => worker::run(I2cBuses::default(), jobs, tx),
            DdcSource::Fake(path) => fake_worker(path, jobs, tx),
        }
    }
}

#[cfg(feature = "fake-ddc")]
fn fake_worker(path: PathBuf, jobs: Receiver<DdcJob>, tx: watch::Sender<Vec<DdcDisplay>>) {
    worker::run(super::file_bus::FileBus::new(path), jobs, tx);
}

/// Without the feature a fake is never made; were one passed, nothing is served (and no real
/// bus is opened in its place).
#[cfg(not(feature = "fake-ddc"))]
fn fake_worker(_path: PathBuf, _jobs: Receiver<DdcJob>, _tx: watch::Sender<Vec<DdcDisplay>>) {}
