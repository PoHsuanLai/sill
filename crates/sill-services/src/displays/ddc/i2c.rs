//! [`DdcBus`] over the kernel's i2c character devices, through the `ddc-i2c` crate: every
//! `/dev/i2c-N` whose adapter belongs to a display controller is asked for its EDID, and one
//! that returns a connected connector's EDID and answers a VCP read is a DDC display.
//!
//! Access is the distribution's business: Fedora's ddcutil package grants the seat's user
//! `/dev/i2c-*` on display adapters (`uaccess`), and `dist/udev/60-sill-ddc.rules` does the same
//! where ddcutil is not installed. Without access every probe comes back empty and brightness keys
//! fall through to nothing, quietly.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ddc::{Ddc, Edid};

use super::bus::{DdcBus, DdcError, DdcId, Endpoint};
use super::matching::{bus_number, connector_for, is_display_class};
use super::vcp::{Feature, Level};
use crate::cosmic_wl::OutputName;
use crate::displays::drm;
use crate::displays::edid;

type Device = ddc_i2c::I2cDeviceDdc;

/// The real buses.
pub struct I2cBuses {
    i2c_dev: PathBuf,
    drm: PathBuf,
    open: BTreeMap<DdcId, Device>,
}

impl std::fmt::Debug for I2cBuses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("I2cBuses")
            .field("open", &self.open.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Default for I2cBuses {
    fn default() -> Self {
        I2cBuses {
            i2c_dev: PathBuf::from("/sys/class/i2c-dev"),
            drm: PathBuf::from("/sys/class/drm"),
            open: BTreeMap::new(),
        }
    }
}

impl DdcBus for I2cBuses {
    fn probe(&mut self) -> Vec<Endpoint> {
        let connectors: Vec<(OutputName, Vec<u8>)> = drm::connected(&self.drm)
            .into_iter()
            .map(|c| (c.name, c.edid))
            .filter(|(_, edid)| !edid.is_empty())
            .collect();
        self.open.clear();
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
            self.open.insert(id, device);
        }
        found
    }

    fn read(&mut self, id: DdcId, feature: Feature) -> Result<Level, DdcError> {
        let device = self.open.get_mut(&id).ok_or(DdcError::Unknown(id))?;
        device
            .get_vcp_feature(feature.code())
            .map(|v| Level {
                value: v.value(),
                max: v.maximum(),
            })
            .map_err(classify)
    }

    fn write(&mut self, id: DdcId, feature: Feature, value: u16) -> Result<(), DdcError> {
        let device = self.open.get_mut(&id).ok_or(DdcError::Unknown(id))?;
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
