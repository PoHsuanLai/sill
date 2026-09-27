//! sill's own idle on the daemon's side (Q420 B; sill FINDINGS "sill idle (Q420 B)", F910): the
//! idle service's config from `idle.*` and `session.lock_grace_s` ([`policy_of`]), sent now and
//! on every change ([`spawn_policy`]). The service resolves the kind of session itself and is
//! `Off` anywhere but a sill session.
//!
//! **Freeze:** [`spawn_policy`] is a no-op (the service stays passive, so `sill run` nested
//! under KDE or on the nested cosmic-comp never dims or powers off a display), and
//! [`policy_of`] is `todo!()` with its table test ignored ("idle fill").

use sill_services::Control;
use sill_services::idle::{IdleCommand, IdleConfig};
use sill_settings::ShellFile;
use tokio::runtime::Handle;
use tokio::sync::watch;

/// The idle config `file` makes: `idle.times` `cosmic` is `TimesChoice::Cosmic`, `sill` the
/// four `*_min` keys times 60 (0 is `Never`); the dim lead and level, the lock, the lock
/// screen's own time and `session.lock_grace_s` in seconds.
pub fn policy_of(file: &ShellFile) -> IdleConfig {
    let _ = file;
    todo!("idle fill: F910")
}

/// Send [`policy_of`] now and on every settings change that changes it (the polkit policy's
/// pattern). Freeze: sends nothing.
pub fn spawn_policy(
    rt: &Handle,
    settings: watch::Receiver<ShellFile>,
    control: Control<IdleCommand>,
) {
    let _ = (rt, settings, control);
}

#[cfg(test)]
mod tests {
    use sill_services::idle::{
        DimLevel, DimPlan, IdleConfig, IdleLock, IdleSecs, IdleTimes, LockGrace, LockedScreenOff,
        PowerPair, Timeout, TimesChoice,
    };
    use sill_settings::{Count, IdleLockAt, IdleTimesFrom, Percent, ShellFile};

    use super::policy_of;

    #[test]
    #[ignore = "idle fill"]
    fn the_config_is_the_settings_in_seconds() {
        let defaults = ShellFile::default();
        let mut own = ShellFile::default();
        own.idle.times = IdleTimesFrom::Sill;
        own.idle.screen_off_ac_min = Count(10);
        own.idle.screen_off_battery_min = Count(2);
        own.idle.suspend_ac_min = Count(0);
        own.idle.suspend_battery_min = Count(20);
        own.idle.dim_s = Count(0);
        own.idle.dim_level_pct = Percent(30);
        own.idle.lock = IdleLockAt::Never;
        own.idle.locked_screen_off_s = Count(0);
        own.session.lock_grace_s = Count(0);
        let after = |s| Timeout::After(IdleSecs(s));
        let cases = [
            (
                defaults,
                IdleConfig {
                    times: TimesChoice::Cosmic,
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
                    times: TimesChoice::Own(IdleTimes {
                        screen_off: PowerPair {
                            ac: after(600),
                            battery: after(120),
                        },
                        suspend: PowerPair {
                            ac: Timeout::Never,
                            battery: after(1_200),
                        },
                    }),
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
            assert_eq!(policy_of(&file), want, "{:?}", file.idle.times);
        }
    }
}
