//! sill's own polkit authentication agent (design/20 §1.10, M11; FINDINGS "M11 freeze"
//! F545-F547), chosen by `session.polkit_agent = "sill"`. The daemon's own process registers
//! `org.freedesktop.PolicyKit1.AuthenticationAgent` for its logind session; polkitd calls
//! `BeginAuthentication` when a program of that session needs an administrator (`pkexec`, a
//! settings page), the service publishes a [`PromptView`] the polkit prompt surface draws, and
//! the typed secret goes to polkit's own helper, never through sill's PAM.
//!
//! How agents authenticate (researched, F545): an agent never checks a password itself, and
//! never needs root. It hands the request's cookie to `polkit-agent-helper-1` (setuid root;
//! `/usr/lib/polkit-1/` on Fedora and Arch, `/usr/libexec/` on Debian), which runs PAM as the
//! chosen identity and, on success, tells polkitd itself (`AuthenticationAgentResponse2`), so a
//! lying agent cannot fake an answer. libpolkit-agent-1's `PolkitAgentSession` (under
//! polkit-gnome, polkit-kde and lxqt-policykit) and cosmic-osd's own Rust version speak the
//! same line protocol: with polkit 124+'s socket-activated helper
//! (`/run/polkit/agent-helper.socket`, `polkit-agent-helper.socket`) the agent connects and
//! writes the user name and the cookie, a line each; otherwise it runs the setuid helper with
//! the user name as its argument and writes the cookie on stdin. The helper then writes
//! `PAM_PROMPT_ECHO_OFF <prompt>`, `PAM_PROMPT_ECHO_ON <prompt>`, `PAM_ERROR_MSG <text>`,
//! `PAM_TEXT_INFO <text>`, and finally `SUCCESS` or `FAILURE`; the agent answers a prompt with
//! one line. On this machine (Fedora 44, polkit 127) the socket unit is installed but disabled,
//! so the setuid helper is the route. The identity asked for is the session's own user when
//! polkit offers it, else root, else the first `unix-user` (what GNOME Shell and cosmic-osd
//! pick). Registration: `RegisterAuthenticationAgent((unix-session, {session-id}), locale,
//! object_path)` on `org.freedesktop.PolicyKit1.Authority`, the session id from logind.
//!
//! **Passive until the daemon's first [`PolkitCommand::Policy`]**: `Services::spawn` in a test
//! never touches the system bus; every live test runs on a private bus with a stand-in
//! Authority and a stand-in helper ([`Setup`]), never polkitd.

mod bus;
mod daemon_setup;
mod dbus;
pub mod helper;
pub mod identity;
mod live;
pub mod machine;
mod run;
mod types;

pub use dbus::{AGENT_PATH, AUTHORITY_NAME, AUTHORITY_PATH};
pub use helper::{HelperLine, HelperOpening, HelperRoute, opening, parse_helper_line};
pub use identity::pick_identity;
pub use machine::{Completion, Origin, PolkitEffect, PolkitEvent, PolkitState, polkit_of, reduce};
pub use types::{
    ActionId, AgentPolicy, AuthRequest, Cookie, Echo, FAKE_SECRET, FakeRequest, Gid, Identity,
    LastTry, Polkit, PolkitCommand, PromptPhase, PromptView, Registration, Uid,
};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// Which bus polkitd is on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PolkitBus {
    /// The system bus (the daemon).
    System,
    /// A private bus by address (tests: a stand-in Authority on it).
    Address(String),
}

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
pub struct Setup {
    pub bus: PolkitBus,
    pub helper: HelperChoice,
    pub session: SessionChoice,
}

impl Setup {
    /// The daemon's: the system bus, polkit's helper, this session.
    pub fn system() -> Setup {
        Setup {
            bus: PolkitBus::System,
            helper: HelperChoice::System,
            session: SessionChoice::Ours,
        }
    }
}

/// Start the agent service on `rt` for the daemon ([`Setup::system`]); passive until a
/// `Policy(Own)`.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Polkit>, Control<PolkitCommand>) {
    spawn_with(rt, daemon_setup::daemon_setup())
}

/// [`spawn`] against `setup`.
pub fn spawn_with(rt: &Handle, setup: Setup) -> (watch::Receiver<Polkit>, Control<PolkitCommand>) {
    let (tx, rx) = watch::channel(Polkit::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(tx, commands, setup));
    (rx, control)
}
