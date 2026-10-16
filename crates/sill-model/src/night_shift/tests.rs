use super::*;
use crate::displays::ddc::{Controls, DdcDisplay, Endpoint, RgbGain};
use crate::ids::{DdcId, OutputName};

fn at(hour: u32, minute: u32) -> MinuteOfDay {
    MinuteOfDay::wrapping(hour * 60 + minute)
}

/// 2026-10-16 00:00:00 UTC in Unix seconds, so `now(h, m)` lies on that day.
const DAY: i64 = 1_792_108_800;

fn now(hour: u32, minute: u32) -> Now {
    Now {
        unix: UnixSeconds(DAY + i64::from(hour * 3600 + minute * 60)),
        minute: at(hour, minute),
    }
}

fn light(schedule: Schedule) -> NightLight {
    NightLight {
        schedule,
        ..NightLight::default()
    }
}

fn ready(light: NightLight, manual: Option<Manual>, backend: Backend) -> NightShift {
    NightShift {
        reach: Reach::Ready,
        light,
        manual,
        backend,
    }
}

#[test]
fn a_manual_turn_on_ends_when_the_night_does() {
    let seven = at(7, 0);
    let cases = [
        (
            "evening: the next morning",
            now(22, 30),
            seven,
            DAY + 31 * 3600,
        ),
        (
            "after midnight: this morning",
            now(2, 10),
            seven,
            DAY + 7 * 3600,
        ),
        (
            "the very minute: a day on",
            now(7, 0),
            seven,
            DAY + 31 * 3600,
        ),
        ("midday", now(12, 0), seven, DAY + 31 * 3600),
        (
            "seconds into the minute do not count",
            Now {
                unix: UnixSeconds(DAY + 79_859),
                minute: at(22, 10),
            },
            seven,
            DAY + 31 * 3600,
        ),
    ];
    for (name, at_now, to, want) in cases {
        assert_eq!(until_tomorrow(at_now, to), UnixSeconds(want), "{name}");
    }
}

#[test]
fn the_switch_turns_the_night_light_on_until_tomorrow_and_off_again() {
    let scheduled = ready(light(Schedule::Scheduled), None, Backend::Software);
    let (on, effects) = step(
        scheduled,
        NightShiftEvent::Told(NightShiftCommand::TurnOnUntilTomorrow, now(23, 0)),
    );
    let manual = Manual {
        until: UnixSeconds(DAY + 31 * 3600),
        restore: light(Schedule::Scheduled),
    };
    assert_eq!(on.light.schedule, Schedule::Always);
    assert_eq!(on.manual, Some(manual));
    assert_eq!(
        effects,
        vec![
            NightShiftEffect::WriteLight(light(Schedule::Always)),
            NightShiftEffect::WriteManual(Some(manual)),
        ]
    );
    assert_eq!(shown(&on), Shown::OnUntilTomorrow);

    let (off, effects) = step(
        on,
        NightShiftEvent::Told(NightShiftCommand::TurnOff, now(23, 5)),
    );
    assert_eq!(
        off.light,
        light(Schedule::Scheduled),
        "the schedule is back"
    );
    assert_eq!(off.manual, None);
    assert_eq!(
        effects,
        vec![
            NightShiftEffect::WriteLight(light(Schedule::Scheduled)),
            NightShiftEffect::WriteManual(None),
        ]
    );
}

#[test]
fn a_manual_turn_on_runs_out_with_the_clock_and_not_before() {
    let (on, _) = step(
        ready(light(Schedule::Off), None, Backend::Software),
        NightShiftEvent::Told(NightShiftCommand::TurnOnUntilTomorrow, now(22, 0)),
    );
    let (still, effects) = step(on, NightShiftEvent::Tick(now(6, 59)));
    assert_eq!(still, on, "a tick too early changes nothing");
    assert!(effects.is_empty());
    let later = Now {
        unix: UnixSeconds(DAY + 31 * 3600 + 1),
        minute: at(7, 0),
    };
    let (over, effects) = step(on, NightShiftEvent::Tick(later));
    assert_eq!(over.light, light(Schedule::Off));
    assert_eq!(over.manual, None);
    assert_eq!(effects.len(), 2);
}

#[test]
fn turning_off_without_a_manual_turn_on_switches_the_light_off() {
    let always = ready(light(Schedule::Always), None, Backend::Software);
    let (off, effects) = step(
        always,
        NightShiftEvent::Told(NightShiftCommand::TurnOff, now(23, 0)),
    );
    assert_eq!(off.light.schedule, Schedule::Off);
    assert_eq!(
        effects,
        vec![NightShiftEffect::WriteLight(light(Schedule::Off))]
    );
    let already = ready(light(Schedule::Off), None, Backend::Software);
    let (same, effects) = step(
        already,
        NightShiftEvent::Told(NightShiftCommand::TurnOff, now(23, 0)),
    );
    assert_eq!((same, effects), (already, vec![]));
}

#[test]
fn the_warmth_is_kept_through_a_manual_turn_on() {
    let (on, _) = step(
        ready(light(Schedule::Scheduled), None, Backend::Software),
        NightShiftEvent::Told(NightShiftCommand::TurnOnUntilTomorrow, now(22, 0)),
    );
    let warmer = Warmth::clamped(2900);
    let (on, effects) = step(
        on,
        NightShiftEvent::Told(NightShiftCommand::SetWarmth(warmer), now(22, 1)),
    );
    assert_eq!(on.light.warmth, warmer);
    assert_eq!(
        effects.len(),
        2,
        "the light and the manual turn-on's restore"
    );
    let (off, _) = step(
        on,
        NightShiftEvent::Told(NightShiftCommand::TurnOff, now(22, 2)),
    );
    assert_eq!(off.light.schedule, Schedule::Scheduled);
    assert_eq!(off.light.warmth, warmer);
}

#[test]
fn nothing_is_written_while_casements_settings_are_out_of_reach() {
    let lost = NightShift::default();
    assert_eq!(lost.reach, Reach::Unavailable);
    for command in [
        NightShiftCommand::TurnOnUntilTomorrow,
        NightShiftCommand::TurnOff,
        NightShiftCommand::SetWarmth(Warmth::NIGHT),
    ] {
        let (same, effects) = step(lost, NightShiftEvent::Told(command, now(22, 0)));
        assert_eq!((same, effects), (lost, vec![]), "{command:?}");
    }
    assert_eq!(shown(&lost), Shown::Unavailable);
    let (ready_again, _) = step(
        lost,
        NightShiftEvent::Loaded {
            light: NightLight::default(),
            manual: None,
        },
    );
    assert_eq!(ready_again.reach, Reach::Ready);
    let (gone, _) = step(ready_again, NightShiftEvent::Unreachable);
    assert_eq!(gone.reach, Reach::Unavailable);
}

#[test]
fn a_change_made_in_settings_ends_the_manual_turn_on() {
    let manual = Manual {
        until: UnixSeconds(DAY + 31 * 3600),
        restore: light(Schedule::Scheduled),
    };
    let kept = step(
        NightShift::default(),
        NightShiftEvent::Loaded {
            light: light(Schedule::Always),
            manual: Some(manual),
        },
    );
    assert_eq!(kept.0.manual, Some(manual), "casement still says all day");
    assert!(kept.1.is_empty());
    let changed = step(
        NightShift::default(),
        NightShiftEvent::Loaded {
            light: light(Schedule::Scheduled),
            manual: Some(manual),
        },
    );
    assert_eq!(changed.0.manual, None, "the person set another schedule");
    assert_eq!(changed.1, vec![NightShiftEffect::WriteManual(None)]);
}

#[test]
fn where_sill_warms_the_monitors_the_software_light_stays_as_it_is() {
    let scheduled = ready(light(Schedule::Scheduled), None, Backend::Software);
    let (told, effects) = step(
        scheduled,
        NightShiftEvent::Told(NightShiftCommand::Backend(Backend::Hardware), now(22, 0)),
    );
    assert_eq!(told.backend, Backend::Hardware);
    assert_eq!(told.light, scheduled.light, "the light is not stood down");
    assert_eq!(effects, vec![]);
    let (on, effects) = step(
        told,
        NightShiftEvent::Told(NightShiftCommand::TurnOnUntilTomorrow, now(22, 0)),
    );
    assert_eq!(
        on.light.schedule,
        Schedule::Always,
        "the switch still warms"
    );
    assert_eq!(effects.len(), 2);
    let (loaded, effects) = step(
        told,
        NightShiftEvent::Loaded {
            light: light(Schedule::Always),
            manual: None,
        },
    );
    assert_eq!(
        loaded.light.schedule,
        Schedule::Always,
        "an edit in Settings stays"
    );
    assert_eq!((loaded.backend, effects), (Backend::Hardware, vec![]));
}

fn monitor(bus: u16, connector: Option<&str>, gain: Option<RgbGain>) -> DdcDisplay {
    DdcDisplay {
        endpoint: Endpoint {
            id: DdcId(bus),
            connector: connector.map(|name| OutputName(name.to_owned())),
            product: format!("Monitor {bus}"),
        },
        controls: Controls {
            gain,
            ..Controls::default()
        },
    }
}

fn names(connectors: &[&str]) -> Vec<OutputName> {
    connectors
        .iter()
        .map(|name| OutputName((*name).to_owned()))
        .collect()
}

#[test]
fn casement_leaves_the_monitors_sill_warms_over_ddc_alone() {
    let gain = Some(RgbGain::default());
    let monitors = [
        monitor(5, Some("HDMI-A-1"), gain),
        monitor(4, Some("DP-1"), gain),
        monitor(6, Some("DP-2"), None),
        monitor(7, None, gain),
        monitor(8, Some("DP-1"), gain),
    ];
    let cases: &[(&str, Backend, &[DdcDisplay], &[&str])] = &[
        ("software: none", Backend::Software, &monitors, &[]),
        (
            "hardware: the connectors with gains, sorted, once each",
            Backend::Hardware,
            &monitors,
            &["DP-1", "HDMI-A-1"],
        ),
        (
            "hardware: a monitor without gains or a connector is not listed",
            Backend::Hardware,
            &monitors[2..4],
            &[],
        ),
        ("hardware: no monitors", Backend::Hardware, &[], &[]),
    ];
    for (name, backend, ddc, want) in cases {
        assert_eq!(skip_outputs(*backend, ddc), names(want), "{name}");
    }
}

#[test]
fn a_monitor_sill_cannot_name_is_left_to_the_software_light() {
    let gain = Some(RgbGain::default());
    let monitors = [
        monitor(7, None, gain),
        monitor(3, None, gain),
        monitor(4, Some("DP-1"), gain),
        monitor(6, None, None),
    ];
    let cases: &[(&str, Backend, &[DdcDisplay], &[u16])] = &[
        (
            "software: nothing to say",
            Backend::Software,
            &monitors,
            &[],
        ),
        (
            "hardware: those with gains and no connector, by bus",
            Backend::Hardware,
            &monitors,
            &[3, 7],
        ),
        (
            "hardware: all matched",
            Backend::Hardware,
            &monitors[2..3],
            &[],
        ),
        ("hardware: no monitors", Backend::Hardware, &[], &[]),
    ];
    for (name, backend, ddc, want) in cases {
        let want: Vec<DdcId> = want.iter().copied().map(DdcId).collect();
        assert_eq!(unmatched(*backend, ddc), want, "{name}");
    }
}

#[test]
fn the_switch_says_what_the_light_is_doing() {
    let on = |schedule, manual| ready(light(schedule), manual, Backend::Software);
    let manual = Manual {
        until: UnixSeconds(1),
        restore: NightLight::default(),
    };
    let cases = [
        ("off", on(Schedule::Off, None), Shown::Off),
        (
            "by hand",
            on(Schedule::Always, Some(manual)),
            Shown::OnUntilTomorrow,
        ),
        ("all day", on(Schedule::Always, None), Shown::On),
        (
            "on a schedule",
            on(Schedule::Scheduled, None),
            Shown::Scheduled {
                from: at(22, 0),
                to: at(7, 0),
            },
        ),
    ];
    for (name, state, want) in cases {
        assert_eq!(shown(&state), want, "{name}");
    }
}

#[test]
fn the_slider_runs_from_less_warm_to_more_warm() {
    let cases = [
        (6500, 0),
        (6800, 0),
        (4600, 500),
        (3400, 815),
        (2700, 1000),
        (1900, 1000),
    ];
    for (kelvin, thousandths) in cases {
        assert_eq!(
            warmth_fraction(Warmth::clamped(kelvin)),
            thousandths,
            "{kelvin} K"
        );
    }
    assert_eq!(warmth_at(0).kelvin(), 6500);
    assert_eq!(warmth_at(500).kelvin(), 4600);
    assert_eq!(warmth_at(1000).kelvin(), 2700);
    assert_eq!(warmth_at(4000).kelvin(), 2700, "past the end");
}
