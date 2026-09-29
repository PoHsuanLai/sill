//! Where the agent lives: polkitd's bus, the helper and the logind session.

use sill_model::polkit_agent::HelperRoute;

use crate::env::Bus;

/// How to reach polkit's helper.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HelperChoice {
    /// Look for the socket, then the setuid binary ([`helper::helper_route`]).
    System,
    /// This route (tests: a stand-in helper program speaking the protocol).
    Use(HelperRoute),
}

/// Which logind session to register for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SessionChoice {
    /// logind's session of this process (`/org/freedesktop/login1/session/auto`).
    Ours,
    /// This id (tests).
    Id(String),
}

/// Where the agent lives.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolkitRoute {
    /// polkitd's bus: the system bus, or a private one with a stand-in Authority (tests).
    pub bus: Bus,
    pub helper: HelperChoice,
    pub session: SessionChoice,
}

impl PolkitRoute {
    /// The daemon's: polkitd on `system`, polkit's helper, this session.
    pub fn system(system: Bus) -> PolkitRoute {
        PolkitRoute {
            bus: system,
            helper: HelperChoice::System,
            session: SessionChoice::Ours,
        }
    }
}
