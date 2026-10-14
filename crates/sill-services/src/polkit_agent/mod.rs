//! sill's own polkit authentication agent (design/20 §1.10). The daemon's own process registers
//! `org.freedesktop.PolicyKit1.AuthenticationAgent` for its logind session; polkitd calls
//! `BeginAuthentication` when a program of that session needs an administrator (`pkexec`, a
//! settings page), the service publishes a `PromptView` the polkit prompt surface draws, and
//! the typed secret goes to polkit's own helper, never through sill's PAM.
//!
//! How agents authenticate: an agent never checks a password itself, and
//! never needs root. It hands the request's cookie to `polkit-agent-helper-1` (setuid root;
//! `/usr/lib/polkit-1/` on Fedora and Arch, `/usr/libexec/` on Debian), which runs PAM as the
//! chosen identity and, on success, tells polkitd itself (`AuthenticationAgentResponse2`), so a
//! lying agent cannot fake an answer. libpolkit-agent-1's `PolkitAgentSession` (under
//! polkit-gnome, polkit-kde and lxqt-policykit) speak the
//! same line protocol: with polkit 124+'s socket-activated helper
//! (`/run/polkit/agent-helper.socket`, `polkit-agent-helper.socket`) the agent connects and
//! writes the user name and the cookie, a line each; otherwise it runs the setuid helper with
//! the user name as its argument and writes the cookie on stdin. The helper then writes
//! `PAM_PROMPT_ECHO_OFF <prompt>`, `PAM_PROMPT_ECHO_ON <prompt>`, `PAM_ERROR_MSG <text>`,
//! `PAM_TEXT_INFO <text>`, and finally `SUCCESS` or `FAILURE`; the agent answers a prompt with
//! one line. On this machine (Fedora 44, polkit 127) the socket unit is installed but disabled,
//! so the setuid helper is the route. The identity asked for is the session's own user when
//! polkit offers it, else root, else the first `unix-user` (what GNOME Shell
//! picks). Registration: `RegisterAuthenticationAgent((unix-session, {session-id}), locale,
//! object_path)` on `org.freedesktop.PolicyKit1.Authority`, the session id from logind.
//!
//! **Passive until the daemon's first [`PolkitCommand::Policy`]**: the daemon's `start` in a test
//! never touches the system bus; every live test runs on a private bus with a stand-in
//! Authority and a stand-in helper ([`PolkitRoute`]), never polkitd.

mod backend;

use sill_model::polkit_agent::{Polkit, PolkitCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use backend::{
    AGENT_PATH, AUTHORITY_NAME, AUTHORITY_PATH, HelperChoice, PolkitRoute, SessionChoice,
};

/// sill's polkit agent as a [`Service`], registering where `env`'s [`PolkitRoute`] says (the
/// daemon's: [`PolkitRoute::system`]); passive until a `Policy(Own)`.
#[derive(Debug)]
pub enum PolkitAgentService {}

impl Service for PolkitAgentService {
    const NAME: &'static str = "polkit_agent";
    type State = Polkit;
    type Command = PolkitCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let route = env.polkit.clone();
        async move {
            backend::run(io.state, io.commands, route).await;
            Ok(())
        }
    }
}
