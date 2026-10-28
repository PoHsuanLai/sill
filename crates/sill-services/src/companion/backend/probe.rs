//! What a tap or a summon finds when it arrives, answered in order: whether the focused app owns
//! its bus name (the companion client of an app is its provider on that name), and whether the
//! app takes the summon it is asked.

use docket_dbus::{IntentProviderProxy, PROVIDER_PATH};
use docket_wire::{SummonAnswer, SummonOrigin, SummonSerial};
use prov::AppName;
use sill_model::companion::{CompanionEvent, FocusFacts, FocusInput};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use zbus::Connection;

use super::text::{from_json, to_json};
use crate::call_error::CallError;
use crate::env::{NameState, name_state};
use crate::seat::SeatView;

/// What arrived and must be looked at before the machines see it.
#[derive(Debug)]
pub(super) enum Arrival {
    /// The double-tap key was tapped.
    Tap(FocusInput),
    /// A summon.
    Summon(FocusInput),
}

/// Looks at each arrival in the order it came, so two quick taps stay two taps in order, and
/// hands the machines what it found.
pub(super) async fn run(
    seat: SeatView,
    mut arrivals: UnboundedReceiver<Arrival>,
    events: UnboundedSender<CompanionEvent>,
) {
    while let Some(arrival) = arrivals.recv().await {
        let (event, input) = match arrival {
            Arrival::Tap(input) => (
                CompanionEvent::Tapped as fn(FocusFacts) -> CompanionEvent,
                input,
            ),
            Arrival::Summon(input) => (
                CompanionEvent::Summoned as fn(FocusFacts) -> CompanionEvent,
                input,
            ),
        };
        let serves = serves(seat.current().as_ref(), input.app.as_ref()).await;
        let facts = FocusFacts {
            window: input.window,
            app: input.app,
            launcher: input.launcher,
            serves_companion: serves,
        };
        if events.send(event(facts)).is_err() {
            return;
        }
    }
}

/// Whether `app` owns its name on the bus `connection` is on.
async fn serves(connection: Option<&Connection>, app: Option<&AppName>) -> bool {
    let (Some(connection), Some(app)) = (connection, app) else {
        return false;
    };
    match name_state(connection, app.as_str()).await {
        Ok(NameState::Owned) => true,
        Ok(NameState::Activatable | NameState::Missing) | Err(_) => false,
    }
}

/// Ask `app` to take the summon `serial`: its answer, or a refusal when it cannot be asked.
pub(super) async fn ask_app(
    connection: Connection,
    app: AppName,
    serial: SummonSerial,
) -> CompanionEvent {
    let answer = summon(&connection, &app, serial)
        .await
        .unwrap_or(SummonAnswer::Declined);
    CompanionEvent::AppAnswered(serial, answer)
}

async fn summon(
    connection: &Connection,
    app: &AppName,
    serial: SummonSerial,
) -> Result<SummonAnswer, CallError> {
    let provider = IntentProviderProxy::builder(connection)
        .destination(app.as_str().to_owned())?
        .path(PROVIDER_PATH)?
        .build()
        .await?;
    let origin = to_json(&SummonOrigin::DoubleTap)?;
    let reply = provider.summon(serial.0, &origin).await?;
    from_json(&reply)
}
