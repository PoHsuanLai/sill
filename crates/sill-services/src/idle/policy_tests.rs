use super::{
    DimLevel, DimPlan, IdleLock, IdleMode, IdlePolicy, IdleSecs, IdleTimes, LockGrace, LockView,
    LockedScreenOff, OffReason, PowerPair, PowerSource, Threshold, Timeout, TimesChoice, Watch,
    effective_times, power_source, resolve_mode, watches,
};
use crate::Charge;
use crate::battery::Battery;
use crate::idle::cosmic::{CosmicIdle, CosmicIdleTimes};
use crate::locker::Host;
use crate::units::Presence;

const fn after(s: u32) -> Timeout {
    Timeout::After(IdleSecs(s))
}

const fn pair(ac: Timeout, battery: Timeout) -> PowerPair {
    PowerPair { ac, battery }
}

fn policy(times: IdleTimes, lead: u32, locked_off: u32) -> IdlePolicy {
    IdlePolicy {
        mode: IdleMode::Own,
        times,
        dim: DimPlan {
            lead: IdleSecs(lead),
            level: DimLevel(50),
        },
        lock: IdleLock::AtScreenOff,
        locked_off: LockedScreenOff(IdleSecs(locked_off)),
        grace: LockGrace(IdleSecs(5)),
    }
}

fn watch(stage: Threshold, after: u32) -> Watch {
    Watch {
        stage,
        after: IdleSecs(after),
    }
}

#[test]
fn sill_owns_idle_only_in_its_own_session_with_no_cosmic_idle() {
    use CosmicIdle::{Absent, Running};
    let cases = [
        (Host::SillSession, Absent, IdleMode::Own),
        (
            Host::SillSession,
            Running,
            IdleMode::Off(OffReason::CosmicIdleRunning),
        ),
        (
            Host::CosmicSession,
            Absent,
            IdleMode::Off(OffReason::CosmicSession),
        ),
        (
            Host::CosmicSession,
            Running,
            IdleMode::Off(OffReason::CosmicSession),
        ),
        (Host::Guest, Absent, IdleMode::Off(OffReason::Guest)),
        (Host::Guest, Running, IdleMode::Off(OffReason::Guest)),
    ];
    for (host, cosmic, want) in cases {
        assert_eq!(resolve_mode(host, cosmic), want, "{host:?} {cosmic:?}");
    }
}

#[test]
fn cosmic_times_use_one_screen_off_time_for_both_sources() {
    let cosmic = CosmicIdleTimes {
        screen_off: after(600),
        suspend_ac: Timeout::Never,
        suspend_battery: after(900),
    };
    let own = IdleTimes {
        screen_off: pair(after(120), after(60)),
        suspend: pair(after(3_600), after(600)),
    };
    assert_eq!(
        effective_times(TimesChoice::Cosmic, cosmic),
        IdleTimes {
            screen_off: pair(after(600), after(600)),
            suspend: pair(Timeout::Never, after(900)),
        }
    );
    assert_eq!(effective_times(TimesChoice::Own(own), cosmic), own);
}

#[test]
fn battery_only_while_discharging() {
    let battery = |presence, charge| Battery {
        presence,
        charge,
        ..Battery::default()
    };
    let cases = [
        (battery(Presence::Absent, Charge::Unknown), PowerSource::Ac),
        (
            battery(Presence::Present, Charge::Charging),
            PowerSource::Ac,
        ),
        (battery(Presence::Present, Charge::Full), PowerSource::Ac),
        (battery(Presence::Present, Charge::Idle), PowerSource::Ac),
        (
            battery(Presence::Present, Charge::Discharging),
            PowerSource::Battery,
        ),
        (
            battery(Presence::Present, Charge::Empty),
            PowerSource::Battery,
        ),
    ];
    for (battery, want) in cases {
        assert_eq!(power_source(&battery), want, "{:?}", battery.charge);
    }
}

#[test]
fn the_watches_follow_power_lock_and_never() {
    let times = IdleTimes {
        screen_off: pair(after(900), after(120)),
        suspend: pair(after(1_800), Timeout::Never),
    };
    let own = policy(times, 5, 60);
    let off = IdlePolicy {
        mode: IdleMode::Off(OffReason::Guest),
        ..own
    };
    let no_dim = policy(times, 0, 0);
    let long_dim = policy(times, 300, 0);
    use LockView::{Locked, Unlocked};
    use PowerSource::{Ac, Battery};
    use Threshold::{Dim, LockedScreenOff as LockedOff, ScreenOff, Suspend};
    let cases: Vec<(&str, IdlePolicy, PowerSource, LockView, Vec<Watch>)> = vec![
        ("off holds nothing", off, Ac, Unlocked, vec![]),
        (
            "on AC",
            own,
            Ac,
            Unlocked,
            vec![
                watch(Dim, 895),
                watch(ScreenOff, 900),
                watch(Suspend, 1_800),
            ],
        ),
        (
            "on battery, suspend never",
            own,
            Battery,
            Unlocked,
            vec![watch(Dim, 115), watch(ScreenOff, 120)],
        ),
        (
            "locked: the lock screen's own time",
            own,
            Ac,
            Locked,
            vec![watch(Dim, 55), watch(LockedOff, 60), watch(Suspend, 1_800)],
        ),
        (
            "no dim",
            no_dim,
            Ac,
            Unlocked,
            vec![watch(ScreenOff, 900), watch(Suspend, 1_800)],
        ),
        (
            "locked with 0: the normal time",
            no_dim,
            Ac,
            Locked,
            vec![watch(LockedOff, 900), watch(Suspend, 1_800)],
        ),
        (
            "a dim longer than the screen-off time starts at 1 s",
            long_dim,
            Battery,
            Unlocked,
            vec![watch(Dim, 1), watch(ScreenOff, 120)],
        ),
    ];
    for (name, policy, power, lock, want) in cases {
        assert_eq!(watches(&policy, power, lock), want, "{name}");
    }
}
