//! DDC/CI is lossy. On the MA270U about one transaction in ten comes back corrupted ("expected
//! DDC/CI length bit"), and a write that follows a failed read can be dropped by the monitor
//! even though the bus accepted it: the protocol has no acknowledgement. So reads are retried,
//! and a write is read back and repeated until the monitor reports the value (what ddcutil's
//! default verification does too).

use super::vcp::{Feature, Level};
use crate::env::ddc::{DdcBus, DdcError, DdcId};

/// Tries per read, and write-and-verify rounds per write.
pub const ATTEMPTS: usize = 3;

/// `feature`'s level, retrying a failed read.
pub fn read<B: DdcBus + ?Sized>(bus: &B, id: DdcId, feature: Feature) -> Result<Level, DdcError> {
    let mut last = Err(DdcError::Unknown(id));
    for _ in 0..ATTEMPTS {
        last = bus.read(id, feature);
        if last.is_ok() {
            break;
        }
    }
    last
}

/// Write `value` until a read-back shows it. A feature that cannot be read back (a
/// write-only control) counts as written once the write itself succeeded.
pub fn write_verified<B: DdcBus + ?Sized>(
    bus: &B,
    id: DdcId,
    feature: Feature,
    value: u16,
) -> Result<(), DdcError> {
    let mut last = Err(DdcError::Unknown(id));
    for _ in 0..ATTEMPTS {
        last = bus.write(id, feature, value);
        if last.is_err() {
            continue;
        }
        match read(bus, id, feature) {
            Ok(level) if level.value == value => return Ok(()),
            Ok(level) => {
                last = Err(DdcError::Protocol(format!(
                    "wrote {value}, the monitor reports {}",
                    level.value
                )));
            }
            Err(_) => return Ok(()),
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::{read, write_verified};
    use crate::displays::ddc::{Feature, Level};
    use crate::env::ddc::DdcId;
    use crate::env::ddc::FakeBus;

    fn bus(fail_reads: usize, drop_writes: usize) -> FakeBus {
        let bus = FakeBus::default();
        if let Ok(mut state) = bus.0.lock() {
            state.values.insert(
                (DdcId(5), Feature::SpeakerVolume),
                Level { value: 20, max: 50 },
            );
            state.fail_reads = fail_reads;
            state.drop_writes = drop_writes;
        }
        bus
    }

    #[test]
    fn reads_retry_up_to_three_times() {
        let want = Ok(Level { value: 20, max: 50 });
        assert_eq!(read(&bus(0, 0), DdcId(5), Feature::SpeakerVolume), want);
        assert_eq!(read(&bus(2, 0), DdcId(5), Feature::SpeakerVolume), want);
        assert!(read(&bus(3, 0), DdcId(5), Feature::SpeakerVolume).is_err());
    }

    #[test]
    fn a_dropped_write_is_repeated_until_the_monitor_reports_it() {
        let cases: &[(usize, Result<(), ()>, u16)] = &[
            (0, Ok(()), 18),
            (1, Ok(()), 18),
            (2, Ok(()), 18),
            (3, Err(()), 20),
        ];
        for &(drop_writes, want, left_at) in cases {
            let fake = bus(0, drop_writes);
            let result = write_verified(&fake, DdcId(5), Feature::SpeakerVolume, 18);
            assert_eq!(result.map_err(|_| ()), want, "{drop_writes} dropped");
            let now = fake
                .0
                .lock()
                .map(|s| s.values[&(DdcId(5), Feature::SpeakerVolume)].value);
            assert_eq!(now.ok(), Some(left_at), "{drop_writes} dropped");
        }
    }
}
