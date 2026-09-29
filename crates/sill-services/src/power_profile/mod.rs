//! The power profile: power-profiles-daemon over the system bus (design/20 §1.5: the control
//! center's power profile `SegmentedControl`).
//!
//! The daemon answers on `org.freedesktop.UPower.PowerProfiles` (power-profiles-daemon 0.20 or
//! later is required); its `ActiveProfile`, `Profiles` and `PerformanceDegraded` properties are
//! spoken through a hand-written `zbus` proxy (no crate needed for three properties).

mod live;
mod model;
mod ppd;

pub use model::{PowerProfiles, Profile, ProfileDegraded, ProfileEvent, reduce_power_profiles};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// What the power profile service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileCommand {
    /// Set `ActiveProfile`; one the daemon does not offer is dropped.
    Set(Profile),
    /// Re-read every property.
    Refresh,
}

/// Start the power profile task on `rt`. Without power-profiles-daemon the receiver stays at
/// `PowerProfiles::default()` (absent), and the control center hides the control.
pub fn spawn(rt: &Handle) -> (watch::Receiver<PowerProfiles>, Control<ProfileCommand>) {
    let (tx, rx) = watch::channel(PowerProfiles::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(tx, commands));
    (rx, control)
}
