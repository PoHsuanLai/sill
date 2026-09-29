//! A [`DdcBus`] of made-up monitors kept in a JSON file, for the acceptance scripts (a `debug`
//! build's `sill run --fake-ddc FILE`) and tests: every read reads the file again, so a script that
//! rewrites a level plays a change made on the monitor's own buttons; every write sets the
//! level in the file and appends `write <id> <feature> <value>` to `<file>.writes`, which the
//! script checks. Nothing here opens a device.
//!
//! ```json
//! {"displays": [{"id": 7, "connector": "Virtual-1", "product": "Fake Monitor",
//!                "brightness": {"value": 40, "max": 100}}]}
//! ```

use std::io::Write as _;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::bus::{DdcBus, DdcError, DdcId, Endpoint};
use crate::displays::ddc::vcp::{Feature, Level};
use sill_model::ids::OutputName;

/// The file's contents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Monitors {
    displays: Vec<Monitor>,
}

/// One made-up monitor: only a brightness.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Monitor {
    id: u16,
    connector: String,
    product: String,
    brightness: Level,
}

/// The bus over the file at `path`.
#[derive(Debug, Clone)]
pub struct FileBus {
    path: PathBuf,
}

impl FileBus {
    pub fn new(path: PathBuf) -> FileBus {
        FileBus { path }
    }

    fn load(&self) -> Result<Monitors, DdcError> {
        let text = std::fs::read_to_string(&self.path)
            .map_err(|e| DdcError::Bus(format!("{}: {e}", self.path.display())))?;
        serde_json::from_str(&text).map_err(|e| DdcError::Protocol(e.to_string()))
    }

    fn save(&self, monitors: &Monitors) -> Result<(), DdcError> {
        let text =
            serde_json::to_string(monitors).map_err(|e| DdcError::Protocol(e.to_string()))?;
        let next = self.path.with_extension("next");
        std::fs::write(&next, text)
            .and_then(|()| std::fs::rename(&next, &self.path))
            .map_err(|e| DdcError::Bus(e.to_string()))
    }

    fn log(&self, id: DdcId, feature: Feature, value: u16) {
        let mut name = self.path.clone().into_os_string();
        name.push(".writes");
        let line = format!("write {} {feature:?} {value}\n", id.0);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(PathBuf::from(name))
            .and_then(|mut file| file.write_all(line.as_bytes()));
    }
}

impl DdcBus for FileBus {
    fn probe(&self) -> Vec<Endpoint> {
        self.load()
            .map(|monitors| {
                monitors
                    .displays
                    .into_iter()
                    .map(|m| Endpoint {
                        id: DdcId(m.id),
                        connector: Some(OutputName(m.connector)),
                        product: m.product,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn read(&self, id: DdcId, feature: Feature) -> Result<Level, DdcError> {
        let monitors = self.load()?;
        let monitor = monitors
            .displays
            .iter()
            .find(|m| m.id == id.0)
            .ok_or(DdcError::Unknown(id))?;
        match feature {
            Feature::Brightness => Ok(monitor.brightness),
            _ => Err(DdcError::Protocol("unsupported VCP code".to_owned())),
        }
    }

    fn write(&self, id: DdcId, feature: Feature, value: u16) -> Result<(), DdcError> {
        match feature {
            Feature::Brightness => {}
            _ => return Err(DdcError::Protocol("unsupported VCP code".to_owned())),
        }
        let mut monitors = self.load()?;
        let monitor = monitors
            .displays
            .iter_mut()
            .find(|m| m.id == id.0)
            .ok_or(DdcError::Unknown(id))?;
        monitor.brightness.value = value.min(monitor.brightness.max);
        self.save(&monitors)?;
        self.log(id, feature, value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::FileBus;
    use crate::displays::ddc::{Feature, Level};
    use crate::env::ddc::{DdcBus, DdcId};

    #[test]
    fn the_file_answers_reads_and_records_writes() {
        let dir = std::env::temp_dir().join(format!("sill-file-bus-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a scratch dir"); // test-only
        let path = dir.join("ddc.json");
        std::fs::write(
            &path,
            r#"{"displays":[{"id":7,"connector":"Virtual-1","product":"Fake",
                "brightness":{"value":40,"max":100}}]}"#,
        )
        .expect("the fixture writes"); // test-only
        let bus = FileBus::new(path);
        let found = bus.probe();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, DdcId(7));
        let level = |value| Ok(Level { value, max: 100 });
        assert_eq!(bus.read(DdcId(7), Feature::Brightness), level(40));
        assert!(bus.read(DdcId(7), Feature::Contrast).is_err());
        assert!(bus.read(DdcId(8), Feature::Brightness).is_err());
        assert_eq!(bus.write(DdcId(7), Feature::Brightness, 63), Ok(()));
        assert_eq!(bus.read(DdcId(7), Feature::Brightness), level(63));
        let writes = std::fs::read_to_string(dir.join("ddc.json.writes"));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(writes.ok().as_deref(), Some("write 7 Brightness 63\n"));
    }
}
