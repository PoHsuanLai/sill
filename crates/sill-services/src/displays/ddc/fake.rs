//! A [`DdcBus`] made of maps, for tests: displays answer from a table, a feature missing from
//! it fails the way an unsupported VCP code does, and every write is logged.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use super::bus::{DdcBus, DdcError, DdcId, Endpoint};
use super::vcp::{Feature, Level};

/// The fake's state, shared with the test that made it.
#[derive(Debug, Default)]
pub struct FakeState {
    pub endpoints: Vec<Endpoint>,
    pub values: BTreeMap<(DdcId, Feature), Level>,
    pub writes: Vec<(DdcId, Feature, u16)>,
    pub reads: usize,
    /// This many reads fail before reads answer again.
    pub fail_reads: usize,
    /// This many writes are accepted and ignored, as a monitor that was busy does.
    pub drop_writes: usize,
}

/// The bus half; clone the `Arc` to watch it.
#[derive(Debug, Clone, Default)]
pub struct FakeBus(pub Arc<Mutex<FakeState>>);

impl FakeBus {
    fn with<T>(&self, f: impl FnOnce(&mut FakeState) -> T) -> T {
        let mut state = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut state)
    }

    /// A snapshot of every write so far.
    pub fn writes(&self) -> Vec<(DdcId, Feature, u16)> {
        self.with(|s| s.writes.clone())
    }
}

impl DdcBus for FakeBus {
    fn probe(&mut self) -> Vec<Endpoint> {
        self.with(|s| s.endpoints.clone())
    }

    fn read(&mut self, id: DdcId, feature: Feature) -> Result<Level, DdcError> {
        self.with(|s| {
            s.reads += 1;
            if s.fail_reads > 0 {
                s.fail_reads -= 1;
                return Err(DdcError::Protocol("expected DDC/CI length bit".to_owned()));
            }
            s.values
                .get(&(id, feature))
                .copied()
                .ok_or_else(|| DdcError::Protocol("unsupported VCP code".to_owned()))
        })
    }

    fn write(&mut self, id: DdcId, feature: Feature, value: u16) -> Result<(), DdcError> {
        self.with(|s| {
            let level = s
                .values
                .get_mut(&(id, feature))
                .ok_or(DdcError::Unknown(id))?;
            if s.drop_writes > 0 {
                s.drop_writes -= 1;
                return Ok(());
            }
            level.value = value;
            s.writes.push((id, feature, value));
            Ok(())
        })
    }
}
