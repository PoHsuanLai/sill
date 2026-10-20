//! What a client asks the daemon: one [`Request`] per connection.

use serde::{Deserialize, Serialize};
use sill_model::actions::Ask;

use crate::act::ActRequest;
use crate::debug::DebugRequest;
use crate::show::ShowRequest;
use crate::status::StatusRequest;

/// One request. A connection carries exactly one, then its [`crate::Response`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Request {
    /// Is the daemon alive? Answered with `Response::Pong`.
    Ping,
    /// Close every surface and exit the daemon.
    Quit,
    /// Re-read the settings without restarting.
    Reload,
    /// Do an action of the shell: lock, a screenshot, a panel, a step of the switcher, a pin
    /// (`sill lock`, `sill screenshot`, `sill launcher toggle`, ...). One request for every
    /// action; what each is lives in `sill_model::actions`.
    Do(Ask),
    /// Open or close a panel with a particular way, or seed it.
    Show(ShowRequest),
    /// Read a state: answered with `Response::Status`.
    Status(StatusRequest),
    /// Change something.
    Act(ActRequest),
    /// Development probes (`dev/accept/*.sh`); answered even in a release build, acted on only
    /// when the daemon was built with the `debug` feature.
    Debug(DebugRequest),
}
