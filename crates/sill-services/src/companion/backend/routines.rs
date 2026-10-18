//! The routines on `org.quire.Companion1.Helpers2`: the list, making one from what the person
//! confirmed (the ones helpers suggest are in `proposals`), pausing, resuming and removing. The companion tells `Changed` after each, and the
//! screen reads the list again then, so the calls with no body say nothing back.

use companion_wire::{RoutineReply, RoutinesView};
use docket_dbus::CompanionHelpers2Proxy;
use sill_model::companion::{HelpersMsg, RoutinesCommand, RoutinesMsg};

use super::helpers::{silent, unreached};
use super::proposals;
use super::text::{from_json, text, to_json};

/// What the companion answered to `command`, as the message the screen takes; none for a call
/// that says nothing back.
pub(super) async fn carry(
    proxy: &CompanionHelpers2Proxy<'_>,
    command: RoutinesCommand,
) -> Option<HelpersMsg> {
    match command {
        RoutinesCommand::List => Some(read_list(proxy).await),
        RoutinesCommand::ListProposals => Some(proposals::list(proxy).await),
        RoutinesCommand::AcceptProposal(id) => Some(proposals::accept(proxy, &id).await),
        RoutinesCommand::DeclineProposal(id) => proposals::decline(proxy, &id).await,
        RoutinesCommand::Add(form) => Some(add(proxy, &form).await),
        RoutinesCommand::Pause(routine) => silent(proxy.pause_routine(&routine.to_string()).await),
        RoutinesCommand::Resume(routine) => {
            silent(proxy.resume_routine(&routine.to_string()).await)
        }
        RoutinesCommand::Remove(routine) => {
            silent(proxy.remove_routine(&routine.to_string()).await)
        }
    }
}

fn told(msg: RoutinesMsg) -> HelpersMsg {
    HelpersMsg::Routines(msg)
}

async fn read_list(proxy: &CompanionHelpers2Proxy<'_>) -> HelpersMsg {
    match proxy
        .routines()
        .await
        .map_err(text)
        .and_then(|body| from_json::<RoutinesView>(&body))
    {
        Ok(view) => told(RoutinesMsg::Listed(view)),
        Err(why) => unreached(&why).unwrap_or(HelpersMsg::Unreached),
    }
}

async fn add(proxy: &CompanionHelpers2Proxy<'_>, form: &companion_wire::RoutineForm) -> HelpersMsg {
    let body = match to_json(form) {
        Ok(body) => body,
        Err(why) => return unreached(&why).unwrap_or(HelpersMsg::Unreached),
    };
    match proxy
        .add_routine(&body)
        .await
        .map_err(text)
        .and_then(|answer| from_json::<RoutineReply>(&answer))
    {
        Ok(reply) => told(RoutinesMsg::Replied(reply)),
        Err(why) => unreached(&why).unwrap_or(HelpersMsg::Unreached),
    }
}
