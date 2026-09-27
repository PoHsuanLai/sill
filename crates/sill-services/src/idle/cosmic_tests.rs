use super::{CosmicIdleTimes, CosmicKey, CosmicValue, cosmic_times, parse_cosmic_value};
use crate::idle::policy::{IdleSecs, Timeout};

const fn after(s: u32) -> Timeout {
    Timeout::After(IdleSecs(s))
}

#[test]
#[ignore = "idle fill"]
fn a_key_file_is_ron_none_or_some_milliseconds() {
    const CASES: &[(&str, CosmicValue)] = &[
        ("None", CosmicValue::Set(Timeout::Never)),
        ("Some(900000)", CosmicValue::Set(after(900))),
        ("Some(900000)\n", CosmicValue::Set(after(900))),
        ("  Some(60000)  ", CosmicValue::Set(after(60))),
        ("Some(1500)", CosmicValue::Set(after(2))),
        ("Some(0)", CosmicValue::Set(Timeout::Never)),
        ("", CosmicValue::Unreadable),
        ("Some(-1)", CosmicValue::Unreadable),
        ("900000", CosmicValue::Unreadable),
        ("Some(\"15m\")", CosmicValue::Unreadable),
    ];
    for &(text, want) in CASES {
        assert_eq!(parse_cosmic_value(text), want, "{text:?}");
    }
}

#[test]
#[ignore = "idle fill"]
fn missing_or_unreadable_keys_fall_back_to_cosmic_idles_defaults() {
    let none = cosmic_times(|_| None);
    assert_eq!(
        none,
        CosmicIdleTimes {
            screen_off: after(900),
            suspend_ac: after(1_800),
            suspend_battery: after(900),
        }
    );
    let some = cosmic_times(|key| match key {
        CosmicKey::ScreenOffTime => Some("Some(300000)".to_owned()),
        CosmicKey::SuspendOnAcTime => Some("None".to_owned()),
        CosmicKey::SuspendOnBatteryTime => Some("garbage".to_owned()),
    });
    assert_eq!(
        some,
        CosmicIdleTimes {
            screen_off: after(300),
            suspend_ac: Timeout::Never,
            suspend_battery: after(900),
        }
    );
}
