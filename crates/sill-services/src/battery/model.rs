//! The battery's state and its one reducer.

use serde::{Deserialize, Serialize};

use sill_model::units::{Minutes, Percent, Presence, Seconds};

/// UPower's display device, as the bar draws it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Battery {
    /// `IsPresent`: a desktop has no battery, and the bar hides the item.
    pub presence: Presence,
    /// `Percentage`, rounded.
    pub level: Percent,
    pub charge: Charge,
    /// `TimeToEmpty`; zero while unknown.
    pub time_to_empty: Seconds,
    /// `TimeToFull`; zero while unknown.
    pub time_to_full: Seconds,
}

/// UPower's `State`, folded to what the icon distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Charge {
    #[default]
    Unknown,
    Charging,
    Discharging,
    Empty,
    Full,
    /// Plugged in and not charging (UPower's pending-charge and pending-discharge).
    Idle,
}

impl Charge {
    /// UPower's `State` number (0 unknown, 1 charging, 2 discharging, 3 empty, 4 fully
    /// charged, 5 pending charge, 6 pending discharge); anything else is `Unknown`.
    pub fn from_upower(state: u32) -> Charge {
        match state {
            1 => Charge::Charging,
            2 => Charge::Discharging,
            3 => Charge::Empty,
            4 => Charge::Full,
            5 | 6 => Charge::Idle,
            _ => Charge::Unknown,
        }
    }
}

/// How long the battery has left, in the direction it is going.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Remaining {
    Unknown,
    ToEmpty(Minutes),
    ToFull(Minutes),
}

impl Battery {
    /// The time left in the direction the charge is going; `Unknown` when UPower has no
    /// estimate for that direction.
    pub fn remaining(&self) -> Remaining {
        let minutes = |s: Seconds| Minutes(u32::try_from(s.0 / 60).unwrap_or(u32::MAX));
        match self.charge {
            Charge::Discharging if self.time_to_empty.0 > 0 => {
                Remaining::ToEmpty(minutes(self.time_to_empty))
            }
            Charge::Charging if self.time_to_full.0 > 0 => {
                Remaining::ToFull(minutes(self.time_to_full))
            }
            _ => Remaining::Unknown,
        }
    }
}

/// One change UPower reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BatteryEvent {
    Presence(Presence),
    Level(Percent),
    Charge(Charge),
    TimeToEmpty(Seconds),
    TimeToFull(Seconds),
    /// UPower left the bus: back to the default, absent.
    Gone,
}

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

#[cfg(test)]
mod tests {
    use super::{Battery, BatteryEvent, Charge, Remaining, step};
    use sill_model::units::{Minutes, Percent, Presence, Seconds};

    fn present(level: u16, charge: Charge) -> Battery {
        Battery {
            presence: Presence::Present,
            level: Percent(level),
            charge,
            ..Battery::default()
        }
    }

    #[test]
    fn each_event_changes_its_own_field_only() {
        let base = present(50, Charge::Discharging);
        let cases: Vec<(&str, BatteryEvent, Battery)> = vec![
            (
                "level",
                BatteryEvent::Level(Percent(49)),
                present(49, Charge::Discharging),
            ),
            (
                "a level over 100 is clamped",
                BatteryEvent::Level(Percent(104)),
                present(100, Charge::Discharging),
            ),
            (
                "charge",
                BatteryEvent::Charge(Charge::Charging),
                present(50, Charge::Charging),
            ),
            (
                "unplugged battery",
                BatteryEvent::Presence(Presence::Absent),
                Battery {
                    presence: Presence::Absent,
                    ..present(50, Charge::Discharging)
                },
            ),
            (
                "time to empty",
                BatteryEvent::TimeToEmpty(Seconds(3600)),
                Battery {
                    time_to_empty: Seconds(3600),
                    ..present(50, Charge::Discharging)
                },
            ),
            (
                "time to full",
                BatteryEvent::TimeToFull(Seconds(60)),
                Battery {
                    time_to_full: Seconds(60),
                    ..present(50, Charge::Discharging)
                },
            ),
            ("upower gone", BatteryEvent::Gone, Battery::default()),
        ];
        for (name, event, want) in cases {
            assert_eq!(step(base.clone(), event), want, "{name}");
        }
    }

    #[test]
    fn upower_states_fold_to_charge() {
        const CASES: &[(u32, Charge)] = &[
            (0, Charge::Unknown),
            (1, Charge::Charging),
            (2, Charge::Discharging),
            (3, Charge::Empty),
            (4, Charge::Full),
            (5, Charge::Idle),
            (6, Charge::Idle),
            (7, Charge::Unknown),
        ];
        for &(state, want) in CASES {
            assert_eq!(Charge::from_upower(state), want, "{state}");
        }
    }

    #[test]
    fn remaining_follows_the_direction_of_charge() {
        let with = |charge, empty, full| Battery {
            time_to_empty: Seconds(empty),
            time_to_full: Seconds(full),
            ..present(50, charge)
        };
        let cases = [
            (
                with(Charge::Discharging, 7260, 0),
                Remaining::ToEmpty(Minutes(121)),
            ),
            (
                with(Charge::Charging, 0, 1800),
                Remaining::ToFull(Minutes(30)),
            ),
            (with(Charge::Discharging, 0, 1800), Remaining::Unknown),
            (with(Charge::Charging, 7260, 0), Remaining::Unknown),
            (with(Charge::Full, 7260, 1800), Remaining::Unknown),
        ];
        for (battery, want) in cases {
            assert_eq!(battery.remaining(), want, "{battery:?}");
        }
    }
}
