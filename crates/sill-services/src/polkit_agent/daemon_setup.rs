//! The daemon's [`Setup`]: [`Setup::system`], except in a build with debug assertions whose
//! environment names a private bus (`dev/accept-polkit.sh`), so the acceptance can run a debug
//! daemon against the stand-in Authority and the stand-in helper and never reach polkitd.
//!
//! - `SILL_POLKIT_BUS`: the private bus's address. Unset or empty: [`Setup::system`].
//! - `SILL_POLKIT_HELPER`: the stand-in helper program. Unset: no helper at all
//!   (`HelperRoute::Missing`, every request fails), never polkit's own.
//! - `SILL_POLKIT_SESSION`: the session id registered for (default `sill-test`); logind is
//!   never asked.
//!
//! A release build reads none of them: its agent is always the system's.

use std::path::PathBuf;

use super::helper::HelperRoute;
use super::{HelperChoice, SessionChoice, Setup};
use crate::env::{Bus, BusRoute};

/// The environment's words, as [`setup_from`] reads them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct TestBusEnv {
    pub bus: Option<String>,
    pub helper: Option<String>,
    pub session: Option<String>,
}

/// The setup `env` asks for.
pub(super) fn setup_from(env: TestBusEnv, system: Bus) -> Setup {
    let Some(address) = env.bus.filter(|a| !a.is_empty()) else {
        return Setup::system(system);
    };
    let route = env
        .helper
        .filter(|p| !p.is_empty())
        .map_or(HelperRoute::Missing, |p| {
            HelperRoute::Setuid(PathBuf::from(p))
        });
    Setup {
        bus: Bus::system(BusRoute::Address(address)),
        helper: HelperChoice::Use(route),
        session: SessionChoice::Id(env.session.unwrap_or_else(|| "sill-test".to_owned())),
    }
}

/// The daemon's setup (module doc).
#[cfg(debug_assertions)]
pub(super) fn daemon_setup(system: Bus) -> Setup {
    let var = |key: &str| std::env::var(key).ok();
    setup_from(
        TestBusEnv {
            bus: var("SILL_POLKIT_BUS"),
            helper: var("SILL_POLKIT_HELPER"),
            session: var("SILL_POLKIT_SESSION"),
        },
        system,
    )
}

/// The daemon's setup: always the system's in a release build.
#[cfg(not(debug_assertions))]
pub(super) fn daemon_setup(system: Bus) -> Setup {
    Setup::system(system)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{TestBusEnv, setup_from};
    use crate::env::{Bus, BusRoute};
    use crate::polkit_agent::helper::HelperRoute;
    use crate::polkit_agent::{HelperChoice, SessionChoice, Setup};

    #[test]
    fn only_a_named_bus_leaves_the_system_and_never_for_polkits_helper() {
        let some = |s: &str| Some(s.to_owned());
        let cases = [
            (
                TestBusEnv::default(),
                Setup::system(Bus::system(BusRoute::Usual)),
            ),
            (
                TestBusEnv {
                    bus: some(""),
                    helper: some("/x/helper"),
                    session: None,
                },
                Setup::system(Bus::system(BusRoute::Usual)),
            ),
            (
                TestBusEnv {
                    bus: some("unix:path=/tmp/b"),
                    helper: None,
                    session: None,
                },
                Setup {
                    bus: Bus::system(BusRoute::Address("unix:path=/tmp/b".to_owned())),
                    helper: HelperChoice::Use(HelperRoute::Missing),
                    session: SessionChoice::Id("sill-test".to_owned()),
                },
            ),
            (
                TestBusEnv {
                    bus: some("unix:path=/tmp/b"),
                    helper: some("/x/helper"),
                    session: some("c7"),
                },
                Setup {
                    bus: Bus::system(BusRoute::Address("unix:path=/tmp/b".to_owned())),
                    helper: HelperChoice::Use(HelperRoute::Setuid(PathBuf::from("/x/helper"))),
                    session: SessionChoice::Id("c7".to_owned()),
                },
            ),
        ];
        for (env, want) in cases {
            assert_eq!(
                setup_from(env.clone(), Bus::system(BusRoute::Usual)),
                want,
                "{env:?}"
            );
        }
    }
}
