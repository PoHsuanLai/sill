//! Where an output's scale is read and set on a compositor that offers the standard output
//! management (`zwlr_output_manager_v1`) and the standard toplevel protocols: the desktop
//! client's output state and an applied configuration, and the route that picks between this
//! and `cosmic-randr` from the compositor's globals.

use desktop_client::client::DesktopClient;
use desktop_client::output::{ConfigOutcome, OutputEvent};
use desktop_client::{ConfigId, Desktop, DesktopCommand, DesktopEvent, Ticket};
use sill_model::displays::randr::Head;
use sill_model::displays::scale::DisplayScale;

use super::sink::{CosmicRandr, ScaleSink};
use crate::desktop_wl::{config_for_scale, heads_of};
use crate::env::WaylandRoute;
use crate::error::ServiceError;

/// The compositor's outputs through its own output management, on a connection of this sink's.
pub struct WlrOutputs {
    client: DesktopClient,
}

impl std::fmt::Debug for WlrOutputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WlrOutputs").finish_non_exhaustive()
    }
}

fn refused(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::Outputs(error.to_string())
}

impl WlrOutputs {
    /// Wait for the compositor's answer to the configuration `id`.
    fn answer(&mut self, id: ConfigId) -> Result<(), ServiceError> {
        self.client.roundtrip().map_err(refused)?;
        let outcome = self
            .client
            .drain_events()
            .into_iter()
            .find_map(|event| match event {
                DesktopEvent::Output(OutputEvent::ConfigResolved {
                    id: resolved,
                    outcome,
                    ..
                }) if resolved == id => Some(outcome),
                _ => None,
            });
        match outcome {
            Some(ConfigOutcome::Succeeded) => Ok(()),
            Some(other) => Err(refused(format!("the compositor answered {other:?}"))),
            None => Err(refused("the compositor gave no answer")),
        }
    }
}

impl ScaleSink for WlrOutputs {
    fn heads(&mut self) -> Result<Vec<Head>, ServiceError> {
        self.client.roundtrip().map_err(refused)?;
        Ok(heads_of(&self.client.state().outputs))
    }

    fn set_scale(&mut self, head: &Head, scale: DisplayScale) -> Result<(), ServiceError> {
        let config =
            config_for_scale(&self.client.state().outputs, &head.name, scale).map_err(refused)?;
        match self
            .client
            .send(DesktopCommand::ApplyOutputs(config))
            .map_err(refused)?
        {
            Ticket::Config(id) => self.answer(id),
            _ => Err(refused("no configuration was started")),
        }
    }
}

/// The sink the compositor's globals select: the native one on the standard protocols,
/// `cosmic-randr` otherwise.
#[derive(Debug)]
pub enum OutputsRoute {
    /// The standard output management.
    Wlr(Box<WlrOutputs>),
    /// COSMIC's tool.
    Cosmic(CosmicRandr),
}

impl OutputsRoute {
    /// Connect to `route`'s compositor and pick by its globals; a compositor that cannot be
    /// reached, or whose descriptions do not bind, gets the tool.
    pub fn pick(route: &WaylandRoute) -> OutputsRoute {
        let client = route
            .connect()
            .ok()
            .and_then(|connection| DesktopClient::new(&connection).ok())
            .filter(|client| {
                // The standard arm or the desktop's own: both list the wlr toplevel manager or the
                // Quire manager; COSMIC's ext list alone is not enough.
                matches!(
                    desktop_client::backend_for(&client.state().protocols),
                    Some(
                        desktop_client::DesktopBackend::Wlr | desktop_client::DesktopBackend::Quire
                    )
                )
            });
        match client {
            Some(client) => OutputsRoute::Wlr(Box::new(WlrOutputs { client })),
            None => OutputsRoute::Cosmic(CosmicRandr),
        }
    }
}

impl ScaleSink for OutputsRoute {
    fn heads(&mut self) -> Result<Vec<Head>, ServiceError> {
        match self {
            OutputsRoute::Wlr(sink) => sink.heads(),
            OutputsRoute::Cosmic(sink) => sink.heads(),
        }
    }

    fn set_scale(&mut self, head: &Head, scale: DisplayScale) -> Result<(), ServiceError> {
        match self {
            OutputsRoute::Wlr(sink) => sink.set_scale(head, scale),
            OutputsRoute::Cosmic(sink) => sink.set_scale(head, scale),
        }
    }
}
