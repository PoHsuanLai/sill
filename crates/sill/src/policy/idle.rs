//! sill's own idle on the daemon's side: the idle service's config from `idle.*` and
//! `session.lock_grace_s` ([`policy_of`]), sent now and on every change ([`IdleBridge`]). The
//! service resolves the kind of session itself and is `Off` anywhere but a sill session.
//!
//! The daemon's [`IdleTimesSource`] can fix the times instead of the settings: a `debug` build's
//! `sill run --idle-times <screen_off_s>,<suspend_s>` (0 never, [`test_times`]) uses those
//! seconds for both power sources, since `dev/accept/idle.sh` needs seconds and the keys count
//! minutes.

use sill_model::idle::{
    DimLevel, DimPlan, IdleCommand, IdleConfig, IdleLock, IdleSecs, IdleTimes, LockGrace,
    LockedScreenOff, PowerPair, Timeout,
};
use sill_services::env::IdleTimesSource;
use sill_settings::{IdleLockAt, Mins, Secs, ShellFile};

use super::Policy;

/// The idle config `file` makes: the four `*_min` keys times 60 (0 is `Never`); the dim lead and level, the lock, the lock
/// screen's own time and `session.lock_grace_s` in seconds. Each value is held to its key's
/// range, so a hand-edited file cannot ask for an hour of grace.
pub fn policy_of(file: &ShellFile) -> IdleConfig {
    let idle = &file.idle;
    let minutes = |count: Mins, max: u16| match count.0.min(max) {
        0 => Timeout::Never,
        m => Timeout::After(IdleSecs(u32::from(m) * 60)),
    };
    let times = IdleTimes {
        screen_off: PowerPair {
            ac: minutes(idle.screen_off_ac_min, 240),
            battery: minutes(idle.screen_off_battery_min, 240),
        },
        suspend: PowerPair {
            ac: minutes(idle.suspend_ac_min, 480),
            battery: minutes(idle.suspend_battery_min, 480),
        },
    };
    let secs = |count: Secs, max: u16| IdleSecs(u32::from(count.0.min(max)));
    IdleConfig {
        times,
        dim: DimPlan {
            lead: secs(idle.dim_s, 60),
            level: DimLevel(idle.dim_level_pct.0.clamp(10, 90)),
        },
        lock: match idle.lock {
            IdleLockAt::ScreenOff => IdleLock::AtScreenOff,
            IdleLockAt::Never => IdleLock::Never,
        },
        locked_off: LockedScreenOff(secs(idle.locked_screen_off_s, 600)),
        grace: LockGrace(secs(file.session.lock_grace_s, 60)),
    }
}

/// `--idle-times`' text as times in seconds for both sources: `<screen_off_s>,<suspend_s>`
/// (0 never); anything else is `None`.
#[cfg(feature = "debug")]
pub fn test_times(text: &str) -> Option<IdleTimes> {
    let (off, suspend) = text.trim().split_once(',')?;
    let secs = |part: &str| {
        part.trim().parse::<u32>().ok().map(|s| match s {
            0 => Timeout::Never,
            s => Timeout::After(IdleSecs(s)),
        })
    };
    let both = |t: Timeout| PowerPair { ac: t, battery: t };
    Some(IdleTimes {
        screen_off: both(secs(off)?),
        suspend: both(secs(suspend)?),
    })
}

/// The idle service's config from the settings, its times fixed by `times` when a dev script
/// asked (the polkit policy's pattern). The service stays passive until the first.
#[derive(Debug, Clone, Copy)]
pub struct IdleBridge {
    pub times: IdleTimesSource,
}

impl Policy for IdleBridge {
    type Command = IdleCommand;
    type Out = IdleConfig;

    fn of(&self, file: &ShellFile) -> IdleConfig {
        let config = policy_of(file);
        match self.times {
            IdleTimesSource::Fixed(fixed) => IdleConfig {
                times: fixed,
                ..config
            },
            IdleTimesSource::Settings => config,
        }
    }

    fn command(config: IdleConfig) -> IdleCommand {
        IdleCommand::Configure(config)
    }
}

#[cfg(test)]
mod tests {
    use sill_model::idle::{
        DimLevel, DimPlan, IdleConfig, IdleLock, IdleSecs, IdleTimes, LockGrace, LockedScreenOff,
        PowerPair, Timeout,
    };
    use sill_settings::{IdleLockAt, Mins, Percent, Secs, ShellFile};

    use super::policy_of;

    #[test]
    fn the_config_is_the_settings_in_seconds() {
        let defaults = ShellFile::default();
        let mut own = ShellFile::default();
        own.idle.screen_off_ac_min = Mins(10);
        own.idle.screen_off_battery_min = Mins(2);
        own.idle.suspend_ac_min = Mins(0);
        own.idle.suspend_battery_min = Mins(20);
        own.idle.dim_s = Secs(0);
        own.idle.dim_level_pct = Percent(30);
        own.idle.lock = IdleLockAt::Never;
        own.idle.locked_screen_off_s = Secs(0);
        own.session.lock_grace_s = Secs(0);
        let after = |s| Timeout::After(IdleSecs(s));
        let cases = [
            (
                defaults,
                IdleConfig {
                    times: IdleTimes {
                        screen_off: PowerPair {
                            ac: after(900),
                            battery: after(900),
                        },
                        suspend: PowerPair {
                            ac: after(1_800),
                            battery: after(900),
                        },
                    },
                    dim: DimPlan {
                        lead: IdleSecs(5),
                        level: DimLevel(50),
                    },
                    lock: IdleLock::AtScreenOff,
                    locked_off: LockedScreenOff(IdleSecs(60)),
                    grace: LockGrace(IdleSecs(5)),
                },
            ),
            (
                own,
                IdleConfig {
                    times: IdleTimes {
                        screen_off: PowerPair {
                            ac: after(600),
                            battery: after(120),
                        },
                        suspend: PowerPair {
                            ac: Timeout::Never,
                            battery: after(1_200),
                        },
                    },
                    dim: DimPlan {
                        lead: IdleSecs(0),
                        level: DimLevel(30),
                    },
                    lock: IdleLock::Never,
                    locked_off: LockedScreenOff(IdleSecs(0)),
                    grace: LockGrace(IdleSecs(0)),
                },
            ),
        ];
        for (file, want) in cases {
            assert_eq!(policy_of(&file), want, "{:?}", file.idle);
        }
    }

    #[cfg(feature = "debug")]
    #[test]
    fn the_test_times_are_two_counts_of_seconds() {
        use super::test_times;
        let both = |t| PowerPair { ac: t, battery: t };
        let after = |s| Timeout::After(IdleSecs(s));
        let cases = [
            (
                "6,40",
                Some(IdleTimes {
                    screen_off: both(after(6)),
                    suspend: both(after(40)),
                }),
            ),
            (
                " 8 , 0 ",
                Some(IdleTimes {
                    screen_off: both(after(8)),
                    suspend: both(Timeout::Never),
                }),
            ),
            ("", None),
            ("6", None),
            ("6,x", None),
            ("-1,5", None),
        ];
        for (text, want) in cases {
            assert_eq!(test_times(text), want, "{text:?}");
        }
    }
}
