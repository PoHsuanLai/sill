//! Where the outputs are read from: the compositor's standard output management
//! (`zwlr_output_manager_v1`), picked from its globals. sill only reads them; the scale of a
//! display is the compositor's to set.

use desktop_client::Desktop;
use desktop_client::client::DesktopClient;
use sill_model::displays::randr::Head;

use crate::desktop_wl::heads_of;
use crate::env::WaylandRoute;
use crate::error::ServiceError;

/// The compositor's outputs on a connection of this route's own, or none.
#[derive(Debug)]
pub enum OutputsRoute {
    /// The standard output management.
    Wlr(Box<WlrOutputs>),
    /// No output management: the outputs are not listed.
    Bare,
}

/// A desktop client connection, read for its output heads.
pub struct WlrOutputs {
    client: DesktopClient,
}

impl std::fmt::Debug for WlrOutputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WlrOutputs").finish_non_exhaustive()
    }
}

impl OutputsRoute {
    /// Connect to `route`'s compositor and pick by its globals; a compositor that cannot be
    /// reached, or whose descriptions do not bind, gets none.
    pub fn pick(route: &WaylandRoute) -> OutputsRoute {
        let client = route
            .connect()
            .ok()
            .and_then(|connection| DesktopClient::new(&connection).ok())
            .filter(|client| {
                // The standard arm or the desktop's own: both list the wlr toplevel manager or the
                // Quire manager; the ext list alone is not enough.
                matches!(
                    desktop_client::backend_for(&client.state().protocols),
                    Some(
                        desktop_client::DesktopBackend::Wlr | desktop_client::DesktopBackend::Quire
                    )
                )
            });
        match client {
            Some(client) => OutputsRoute::Wlr(Box::new(WlrOutputs { client })),
            None => OutputsRoute::Bare,
        }
    }

    /// Every output the compositor reports now; none without output management.
    pub fn heads(&mut self) -> Result<Vec<Head>, ServiceError> {
        match self {
            OutputsRoute::Wlr(outputs) => {
                outputs
                    .client
                    .roundtrip()
                    .map_err(|error| ServiceError::Outputs(error.to_string()))?;
                Ok(heads_of(&outputs.client.state().outputs))
            }
            OutputsRoute::Bare => Ok(Vec::new()),
        }
    }
}
