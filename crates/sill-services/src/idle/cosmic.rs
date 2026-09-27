//! COSMIC's idle config, read (never written) so COSMIC Settings' Power page keeps working in a
//! sill session (sill FINDINGS "sill idle (Q420 B)", F901). cosmic-config keeps one file per key
//! under `<dir>/com.system76.CosmicIdle/v1/<key>`, each a RON value: `Some(900000)` (ms) or
//! `None` (never). The user's directory (`$XDG_CONFIG_HOME/cosmic`) wins over the system's
//! (`/usr/share/cosmic`); a missing or unreadable key falls back to cosmic-idle's own default
//! (cosmic-idle-config at `epoch-1.8.0`: screen off 15 min, suspend 30 min on AC and 15 min on
//! battery). Pure: the reads come in as text.

use super::policy::{IdleSecs, Timeout};

/// The config's id and version directory.
pub const COSMIC_IDLE_ID: &str = "com.system76.CosmicIdle";
pub const COSMIC_IDLE_VERSION: &str = "v1";

/// The three keys, as cosmic-idle names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CosmicKey {
    ScreenOffTime,
    SuspendOnAcTime,
    SuspendOnBatteryTime,
}

impl CosmicKey {
    pub const ALL: [CosmicKey; 3] = [
        CosmicKey::ScreenOffTime,
        CosmicKey::SuspendOnAcTime,
        CosmicKey::SuspendOnBatteryTime,
    ];

    /// The key's file name.
    pub fn file_name(self) -> &'static str {
        match self {
            CosmicKey::ScreenOffTime => "screen_off_time",
            CosmicKey::SuspendOnAcTime => "suspend_on_ac_time",
            CosmicKey::SuspendOnBatteryTime => "suspend_on_battery_time",
        }
    }

    /// cosmic-idle's compiled default.
    pub fn fallback(self) -> Timeout {
        let minutes = |m: u32| Timeout::After(IdleSecs(m * 60));
        match self {
            CosmicKey::ScreenOffTime | CosmicKey::SuspendOnBatteryTime => minutes(15),
            CosmicKey::SuspendOnAcTime => minutes(30),
        }
    }
}

/// COSMIC's three idle times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CosmicIdleTimes {
    /// One screen-off time for AC and battery alike.
    pub screen_off: Timeout,
    pub suspend_ac: Timeout,
    pub suspend_battery: Timeout,
}

/// One key's file, parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CosmicValue {
    Set(Timeout),
    /// Not RON's `None` or `Some(<u32 ms>)`: the fallback applies.
    Unreadable,
}

/// Whether a cosmic-idle process of this user is running (a `/proc` scan, as the locker's
/// adoption does).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CosmicIdle {
    Running,
    Absent,
}

/// One key file's text: `None` is `Never`; `Some(n)` is `n` milliseconds rounded up to whole
/// seconds (and `Some(0)` is `Never`, as cosmic-idle treats a zero timer); surrounding
/// whitespace is ignored; anything else is `Unreadable`.
pub fn parse_cosmic_value(text: &str) -> CosmicValue {
    let _ = text;
    todo!("idle fill: F901")
}

/// The three times from `read` (a key's text from the user's directory, else the system's, else
/// `None`), each falling back to [`CosmicKey::fallback`] when missing or unreadable.
pub fn cosmic_times(read: impl Fn(CosmicKey) -> Option<String>) -> CosmicIdleTimes {
    let _ = read;
    todo!("idle fill: F901")
}

#[cfg(test)]
#[path = "cosmic_tests.rs"]
mod tests;
