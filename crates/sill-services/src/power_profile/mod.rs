//! The power profile: power-profiles-daemon over the system bus (design/20 §1.5: the control
//! center's power profile `SegmentedControl`).
//!
//! The daemon answers on `org.freedesktop.UPower.PowerProfiles` (power-profiles-daemon 0.20 or
//! later is required); its `ActiveProfile`, `Profiles` and `PerformanceDegraded` properties are
//! spoken through a hand-written `zbus` proxy (no crate needed for three properties).

mod backend;

use sill_model::power_profile::{PowerProfiles, ProfileCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// power-profiles-daemon as a [`Service`]. Without it the state stays at
/// `PowerProfiles::default()` (absent), and the control center hides the control.
#[derive(Debug)]
pub enum PowerProfileService {}

impl Service for PowerProfileService {
    const NAME: &'static str = "power_profile";
    type State = PowerProfiles;
    type Command = ProfileCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        backend::run(env.buses.system.clone(), io.state, io.commands)
    }
}
