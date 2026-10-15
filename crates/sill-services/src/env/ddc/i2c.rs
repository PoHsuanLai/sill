//! [`DdcBus`] over the kernel's i2c character devices, through the `ddc-i2c` crate: every
//! `/dev/i2c-N` whose adapter belongs to a display controller is asked for its EDID, and one
//! that returns a connected connector's EDID and answers a VCP read is a DDC display.
//!
//! Access is a udev matter: a ddcutil package that ships a `uaccess` rule for `/dev/i2c-*` on
//! display adapters (Fedora's does) grants the seat's user access, and `dist/install.sh` puts
//! `dist/udev/60-sill-ddc.rules` in place, with the `i2c-dev` module load, where no ddcutil rule
//! exists. Without access every probe comes back empty and brightness keys
//! fall through to nothing, quietly.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use ddc::{Ddc, Edid};
use sill_model::displays::ddc::Endpoint;
use sill_model::displays::ddc::matching::{bus_number, connector_for, is_display_class};
use sill_model::displays::ddc::vcp::{Feature, Level};
use sill_model::displays::edid;
use sill_model::ids::{DdcId, OutputName};

use super::bus::{DdcBus, DdcError};

type Device = ddc_i2c::I2cDeviceDdc;

/// The real buses.
pub struct I2cBuses {
    i2c_dev: PathBuf,
    drm: PathBuf,
    /// The devices the last probe found answering; only the worker thread takes the lock.
    open: Mutex<BTreeMap<DdcId, Device>>,
}

impl std::fmt::Debug for I2cBuses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("I2cBuses")
            .field("open", &self.devices().keys().copied().collect::<Vec<_>>())
            .finish()
    }
}

impl Default for I2cBuses {
    fn default() -> Self {
        I2cBuses {
            i2c_dev: PathBuf::from("/sys/class/i2c-dev"),
            drm: PathBuf::from("/sys/class/drm"),
            open: Mutex::new(BTreeMap::new()),
        }
    }
}

impl I2cBuses {
    /// The open devices; a poisoned lock still yields them.
    fn devices(&self) -> MutexGuard<'_, BTreeMap<DdcId, Device>> {
        self.open
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl DdcBus for I2cBuses {
    fn probe(&self) -> Vec<Endpoint> {
        let connectors: Vec<(OutputName, Vec<u8>)> = super::connected(&self.drm)
            .into_iter()
            .map(|c| (c.name, c.edid))
            .filter(|(_, edid)| !edid.is_empty())
            .collect();
        let mut open = self.devices();
        open.clear();
        let mut found = Vec::new();
        for id in display_buses(&self.i2c_dev) {
            let Ok(mut device) = ddc_i2c::from_i2c_device(format!("/dev/i2c-{}", id.0)) else {
                continue;
            };
            let mut block = [0u8; 128];
            if device.read_edid(0, &mut block).is_err() {
                continue;
            }
            let Some(connector) = connector_for(&block, &connectors).cloned() else {
                continue;
            };
            if device.get_vcp_feature(Feature::Brightness.code()).is_err()
                && device.get_vcp_feature(Feature::Contrast.code()).is_err()
            {
                continue;
            }
            found.push(Endpoint {
                id,
                connector: Some(connector),
                product: edid::parse(&block).map(|e| e.name).unwrap_or_default(),
            });
            open.insert(id, device);
        }
        found
    }

    fn read(&self, id: DdcId, feature: Feature) -> Result<Level, DdcError> {
        let mut open = self.devices();
        let device = open.get_mut(&id).ok_or(DdcError::Unknown(id))?;
        device
            .get_vcp_feature(feature.code())
            .map(|v| Level {
                value: v.value(),
                max: v.maximum(),
            })
            .map_err(classify)
    }

    fn write(&self, id: DdcId, feature: Feature, value: u16) -> Result<(), DdcError> {
        let mut open = self.devices();
        let device = open.get_mut(&id).ok_or(DdcError::Unknown(id))?;
        device
            .set_vcp_feature(feature.code(), value)
            .map_err(classify)
    }
}

fn classify<E: std::fmt::Display>(error: ddc_i2c::Error<E>) -> DdcError {
    match error {
        ddc_i2c::Error::I2c(e) => DdcError::Bus(e.to_string()),
        ddc_i2c::Error::Ddc(e) => DdcError::Protocol(e.to_string()),
    }
}

/// The i2c buses whose adapter hangs off a display controller, in number order: the adapter's
/// nearest ancestor with a PCI `class` file says what it is.
fn display_buses(i2c_dev: &Path) -> Vec<DdcId> {
    let Ok(entries) = std::fs::read_dir(i2c_dev) else {
        return Vec::new();
    };
    let mut ids: Vec<DdcId> = entries
        .flatten()
        .filter_map(|entry| {
            let id = bus_number(&entry.file_name().to_string_lossy())?;
            let adapter = std::fs::canonicalize(entry.path().join("device")).ok()?;
            let class = adapter
                .ancestors()
                .take(8)
                .find_map(|dir| std::fs::read_to_string(dir.join("class")).ok())?;
            is_display_class(&class).then_some(id)
        })
        .collect();
    ids.sort();
    ids
}
