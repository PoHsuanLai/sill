//! What the settings ask of idle ([`IdleConfig`], from the daemon), what that resolves to in
//! this session ([`IdlePolicy`]), and the pure functions between them: [`resolve_mode`] (who
//! owns idle here), [`effective_times`] (COSMIC's times or sill's), [`power_source`] (AC or
//! battery from the battery service) and [`watches`] (which `ext-idle-notify-v1` notifications
//! to hold). sill FINDINGS "sill idle (Q420 B)" F900-F906.

use crate::battery::{Battery, Charge};
use crate::locker::Host;
use crate::units::Presence;

use super::cosmic::{CosmicIdle, CosmicIdleTimes};

/// Whole seconds of idleness (the unit every idle time is kept in; the settings' minutes are
/// multiplied out once, in the daemon's `policy_of`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct IdleSecs(pub u32);

/// One idle time: never, or after so long without input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Timeout {
    Never,
    After(IdleSecs),
}

/// A time for each power source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PowerPair {
    pub ac: Timeout,
    pub battery: Timeout,
}

/// The screen-off and suspend times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdleTimes {
    pub screen_off: PowerPair,
    pub suspend: PowerPair,
}

/// `idle.times`: whose times apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimesChoice {
    /// COSMIC's `com.system76.CosmicIdle` (COSMIC Settings' Power page), read by the service.
    Cosmic,
    /// sill's own four keys.
    Own(IdleTimes),
}

/// How bright a dimmed display stays, in percent of its brightness (`idle.dim_level_pct`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DimLevel(pub u8);

/// The warning before the displays go off: dimmed for `lead` first (`idle.dim_s`; zero: no
/// dim). Drawn as a sill overlay, never by changing a backlight or a monitor's brightness (F902).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DimPlan {
    pub lead: IdleSecs,
    pub level: DimLevel,
}

/// `idle.lock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdleLock {
    /// Lock as the displays go off for idleness.
    AtScreenOff,
    /// Never lock for idleness (a sleep still locks: the locker's own rule).
    Never,
}

/// `session.lock_grace_s`: after an idle screen-off lock, a wake within this opens the lock
/// without the password; zero always asks (F908).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LockGrace(pub IdleSecs);

/// How long the lock screen may sit untouched before the displays go off again
/// (`idle.locked_screen_off_s`; zero: the normal screen-off time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LockedScreenOff(pub IdleSecs);

/// Everything the daemon reads from the settings for idle (`crate::idle::IdleCommand::Configure`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdleConfig {
    pub times: TimesChoice,
    pub dim: DimPlan,
    pub lock: IdleLock,
    pub locked_off: LockedScreenOff,
    pub grace: LockGrace,
}

/// Whether sill runs idle here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdleMode {
    /// sill does nothing about idle, and why.
    Off(OffReason),
    /// sill dims, turns the displays off, locks and suspends.
    Own,
}

impl Default for IdleMode {
    fn default() -> Self {
        IdleMode::Off(OffReason::NoPolicy)
    }
}

/// Why [`IdleMode::Off`] was chosen (for the log and a later `sill idle status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OffReason {
    /// No policy yet (the passive start).
    #[default]
    NoPolicy,
    /// cosmic-session is running: its cosmic-idle owns idle.
    CosmicSession,
    /// Nested in another desktop's session (development): never touch its displays.
    Guest,
    /// A cosmic-idle process of this user is running in a sill session: sill defers to it.
    CosmicIdleRunning,
}

/// The resolved policy the reducer works from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdlePolicy {
    pub mode: IdleMode,
    pub times: IdleTimes,
    pub dim: DimPlan,
    pub lock: IdleLock,
    pub locked_off: LockedScreenOff,
    pub grace: LockGrace,
}

/// AC or battery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PowerSource {
    #[default]
    Ac,
    Battery,
}

/// One idle notification to hold (`ext_idle_notifier_v1.get_idle_notification`, which honours
/// the compositor's idle inhibitors; never `get_input_idle_notification`, F905).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Watch {
    pub stage: Threshold,
    pub after: IdleSecs,
}

/// What an idle notification is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Threshold {
    /// Start the dim (`screen_off - dim.lead`, when `lead` is not zero).
    Dim,
    /// Displays off (and the idle lock).
    ScreenOff,
    /// The lock screen sat untouched: displays off again.
    LockedScreenOff,
    /// Suspend through logind.
    Suspend,
}

/// Whether the session is locked, as the watches need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockView {
    Unlocked,
    Locked,
}

/// Who owns idle here: `Own` only in a sill session with no cosmic-idle running; else `Off`
/// with the reason (cosmic-session first, then a guest session, then a running cosmic-idle).
pub fn resolve_mode(host: Host, cosmic_idle: CosmicIdle) -> IdleMode {
    match (host, cosmic_idle) {
        (Host::CosmicSession, _) => IdleMode::Off(OffReason::CosmicSession),
        (Host::Guest, _) => IdleMode::Off(OffReason::Guest),
        (Host::SillSession, CosmicIdle::Running) => IdleMode::Off(OffReason::CosmicIdleRunning),
        (Host::SillSession, CosmicIdle::Absent) => IdleMode::Own,
    }
}

/// The times in force: sill's own, or COSMIC's (one screen-off time for both sources).
pub fn effective_times(choice: TimesChoice, cosmic: CosmicIdleTimes) -> IdleTimes {
    match choice {
        TimesChoice::Own(times) => times,
        TimesChoice::Cosmic => IdleTimes {
            screen_off: PowerPair {
                ac: cosmic.screen_off,
                battery: cosmic.screen_off,
            },
            suspend: PowerPair {
                ac: cosmic.suspend_ac,
                battery: cosmic.suspend_battery,
            },
        },
    }
}

/// AC or battery from the battery service: `Battery` while discharging (or empty), else `Ac`;
/// a machine with no battery is always on AC.
pub fn power_source(battery: &Battery) -> PowerSource {
    match (battery.presence, battery.charge) {
        (Presence::Present, Charge::Discharging | Charge::Empty) => PowerSource::Battery,
        (Presence::Absent, _)
        | (Presence::Present, Charge::Unknown | Charge::Charging | Charge::Full | Charge::Idle) => {
            PowerSource::Ac
        }
    }
}

/// The notifications to hold for `policy` on `power` while the session is `lock`: none when the
/// mode is `Off`; `Dim` and `ScreenOff` (or `LockedScreenOff` while locked, when set), and
/// `Suspend`, each only when its time is not `Never`; a dim lead longer than the screen-off time
/// dims from the start of idleness (after zero seconds is never asked: at least one second).
pub fn watches(policy: &IdlePolicy, power: PowerSource, lock: LockView) -> Vec<Watch> {
    match policy.mode {
        IdleMode::Off(_) => return Vec::new(),
        IdleMode::Own => {}
    }
    let off = match lock {
        LockView::Unlocked => (Threshold::ScreenOff, policy.times.screen_off.on(power)),
        LockView::Locked => (Threshold::LockedScreenOff, locked_off(policy, power)),
    };
    let dim = match (policy.dim.lead, off.1) {
        (IdleSecs(0), _) | (_, Timeout::Never) => None,
        (IdleSecs(lead), Timeout::After(IdleSecs(at))) => Some(Watch {
            stage: Threshold::Dim,
            after: IdleSecs(at.saturating_sub(lead).max(1)),
        }),
    };
    let after = |(stage, time): (Threshold, Timeout)| match time {
        Timeout::Never => None,
        Timeout::After(after) => Some(Watch { stage, after }),
    };
    let suspend = after((Threshold::Suspend, policy.times.suspend.on(power)));
    dim.into_iter().chain(after(off)).chain(suspend).collect()
}

/// The lock screen's own screen-off time, or the normal one when it is zero.
fn locked_off(policy: &IdlePolicy, power: PowerSource) -> Timeout {
    match policy.locked_off {
        LockedScreenOff(IdleSecs(0)) => policy.times.screen_off.on(power),
        LockedScreenOff(secs) => Timeout::After(secs),
    }
}

impl PowerPair {
    /// The time for `power`.
    pub fn on(self, power: PowerSource) -> Timeout {
        match power {
            PowerSource::Ac => self.ac,
            PowerSource::Battery => self.battery,
        }
    }
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
