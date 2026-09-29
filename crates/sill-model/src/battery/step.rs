//! The battery's one reducer.

use super::{Battery, BatteryEvent};
use crate::units::Percent;

/// `battery` after `event`.
pub fn step(battery: Battery, event: BatteryEvent) -> Battery {
    match event {
        BatteryEvent::Presence(presence) => Battery {
            presence,
            ..battery
        },
        BatteryEvent::Level(level) => Battery {
            level: Percent(level.0.min(100)),
            ..battery
        },
        BatteryEvent::Charge(charge) => Battery { charge, ..battery },
        BatteryEvent::TimeToEmpty(time_to_empty) => Battery {
            time_to_empty,
            ..battery
        },
        BatteryEvent::TimeToFull(time_to_full) => Battery {
            time_to_full,
            ..battery
        },
        BatteryEvent::Gone => Battery::default(),
    }
}
