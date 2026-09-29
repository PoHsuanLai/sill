use super::{Battery, BatteryEvent, Charge, Remaining, step};
use crate::units::{Minutes, Percent, Presence, Seconds};

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
