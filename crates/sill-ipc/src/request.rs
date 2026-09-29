//! What a client asks the daemon: one [`Request`] per connection.

use serde::{Deserialize, Serialize};

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
    /// Open, close or toggle a panel.
    Show(ShowRequest),
    /// Read a state: answered with `Response::Status`.
    Status(StatusRequest),
    /// Change something.
    Act(ActRequest),
    /// Development probes (`dev/accept-*.sh`); answered even in a release build, acted on only
    /// when the daemon was built with the `debug` feature.
    Debug(DebugRequest),
}
