//! The transition: what each event does to Night Shift, and which files it then asks to change.

use super::{
    Backend, Manual, MinuteOfDay, NightLight, NightShift, NightShiftCommand, NightShiftEffect,
    NightShiftEvent, Now, Reach, Schedule, UnixSeconds,
};

/// When a turn-on made at `now` ends: the next time the clock reads `to` (the end of the night),
/// a whole day on when it reads it now.
pub fn until_tomorrow(now: Now, to: MinuteOfDay) -> UnixSeconds {
    let ahead = (u32::from(to.minutes()) + 1440 - u32::from(now.minute.minutes())) % 1440;
    let ahead = if ahead == 0 { 1440 } else { ahead };
    let minute_start = now.unix.0 - now.unix.0.rem_euclid(60);
    UnixSeconds(minute_start + i64::from(ahead) * 60)
}

/// `state` after `event`, and the writes it asks for.
pub fn step(state: NightShift, event: NightShiftEvent) -> (NightShift, Vec<NightShiftEffect>) {
    match event {
        NightShiftEvent::Loaded { light, manual } => loaded(state, light, manual),
        NightShiftEvent::Unreachable => (
            NightShift {
                reach: Reach::Unavailable,
                ..state
            },
            Vec::new(),
        ),
        NightShiftEvent::Told(command, now) => told(state, command, now),
        NightShiftEvent::Tick(now) => match state.manual {
            Some(manual) if now.unix >= manual.until => ended(state, manual),
            Some(_) | None => (state, Vec::new()),
        },
    }
}

/// casement's settings were read. A manual turn-on lasts only while casement still says "all
/// day": someone who changed the night light meanwhile (in Settings) has ended it.
fn loaded(
    state: NightShift,
    light: NightLight,
    manual: Option<Manual>,
) -> (NightShift, Vec<NightShiftEffect>) {
    let kept = manual.filter(|_| light.schedule == Schedule::Always);
    let mut effects = Vec::new();
    if manual.is_some() && kept.is_none() {
        effects.push(NightShiftEffect::WriteManual(None));
    }
    let next = NightShift {
        reach: Reach::Ready,
        light,
        manual: kept,
        backend: state.backend,
    };
    let (next, quiet) = quiet_for_hardware(next);
    effects.extend(quiet);
    (next, effects)
}

fn told(
    state: NightShift,
    command: NightShiftCommand,
    now: Now,
) -> (NightShift, Vec<NightShiftEffect>) {
    if let NightShiftCommand::Backend(backend) = command {
        return quiet_for_hardware(NightShift { backend, ..state });
    }
    let reachable = state.reach == Reach::Ready;
    match (command, reachable, state.backend) {
        (_, false, _) | (NightShiftCommand::TurnOnUntilTomorrow, true, Backend::Hardware) => {
            (state, Vec::new())
        }
        (NightShiftCommand::TurnOnUntilTomorrow, true, Backend::Software) => turned_on(state, now),
        (NightShiftCommand::TurnOff, true, _) => match state.manual {
            Some(manual) => ended(state, manual),
            None => turned_off(state),
        },
        (NightShiftCommand::SetWarmth(warmth), true, _) => {
            let light = NightLight {
                warmth,
                ..state.light
            };
            let manual = state.manual.map(|manual| Manual {
                restore: NightLight {
                    warmth,
                    ..manual.restore
                },
                ..manual
            });
            let mut effects = vec![NightShiftEffect::WriteLight(light)];
            if manual.is_some() {
                effects.push(NightShiftEffect::WriteManual(manual));
            }
            (
                NightShift {
                    light,
                    manual,
                    ..state
                },
                effects,
            )
        }
        (NightShiftCommand::Backend(_), true, _) => (state, Vec::new()),
    }
}

/// All day now, until the end of the night; what it was is kept to be put back (a second press
/// keeps the first's).
fn turned_on(state: NightShift, now: Now) -> (NightShift, Vec<NightShiftEffect>) {
    let restore = state.manual.map_or(state.light, |manual| manual.restore);
    let manual = Manual {
        until: until_tomorrow(now, restore.to),
        restore,
    };
    let light = NightLight {
        schedule: Schedule::Always,
        ..state.light
    };
    (
        NightShift {
            light,
            manual: Some(manual),
            ..state
        },
        vec![
            NightShiftEffect::WriteLight(light),
            NightShiftEffect::WriteManual(Some(manual)),
        ],
    )
}

/// The night light off (nothing manual to end).
fn turned_off(state: NightShift) -> (NightShift, Vec<NightShiftEffect>) {
    match state.light.schedule {
        Schedule::Off => (state, Vec::new()),
        Schedule::Always | Schedule::Scheduled => {
            let light = NightLight {
                schedule: Schedule::Off,
                ..state.light
            };
            (
                NightShift { light, ..state },
                vec![NightShiftEffect::WriteLight(light)],
            )
        }
    }
}

/// A manual turn-on is over, by the person or by the clock: casement is put back as it was, with
/// the warmth the person has chosen since.
fn ended(state: NightShift, manual: Manual) -> (NightShift, Vec<NightShiftEffect>) {
    let light = NightLight {
        warmth: state.light.warmth,
        ..manual.restore
    };
    (
        NightShift {
            light,
            manual: None,
            ..state
        },
        vec![
            NightShiftEffect::WriteLight(light),
            NightShiftEffect::WriteManual(None),
        ],
    )
}

/// Where sill warms the displays itself, casement stays off, so the screen is not warmed twice.
fn quiet_for_hardware(state: NightShift) -> (NightShift, Vec<NightShiftEffect>) {
    match (state.backend, state.light.schedule, state.reach) {
        (Backend::Hardware, Schedule::Always | Schedule::Scheduled, Reach::Ready) => {
            let light = NightLight {
                schedule: Schedule::Off,
                ..state.light
            };
            let mut effects = vec![NightShiftEffect::WriteLight(light)];
            if state.manual.is_some() {
                effects.push(NightShiftEffect::WriteManual(None));
            }
            (
                NightShift {
                    light,
                    manual: None,
                    ..state
                },
                effects,
            )
        }
        (Backend::Hardware | Backend::Software, _, _) => (state, Vec::new()),
    }
}
