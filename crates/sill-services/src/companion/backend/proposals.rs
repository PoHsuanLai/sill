//! The routines helpers suggest, on `org.quire.Companion1.Helpers2`: the list, accepting one
//! (answered like making a routine) and turning one down. This file is the only place that names
//! the suggestion calls. The companion tells `Changed` when the list changes.

use companion_wire::{ProposalId, RoutineProposal, RoutineReply};
use docket_dbus::CompanionHelpers2Proxy;
use sill_model::companion::{HelpersMsg, RoutinesMsg};

use super::helpers::{silent, unreached};
use super::text::{from_json, text};

/// What the suggested routines are now.
pub(super) async fn list(proxy: &CompanionHelpers2Proxy<'_>) -> HelpersMsg {
    match proxy
        .routine_proposals()
        .await
        .map_err(text)
        .and_then(|body| from_json::<Vec<RoutineProposal>>(&body))
    {
        Ok(proposals) => HelpersMsg::Routines(RoutinesMsg::Proposed(proposals)),
        Err(why) => unreached(&why).unwrap_or(HelpersMsg::Unreached),
    }
}

/// Make the routine `id` suggested; the companion's answer is the same as to making one.
pub(super) async fn accept(proxy: &CompanionHelpers2Proxy<'_>, id: &ProposalId) -> HelpersMsg {
    match proxy
        .accept_routine_proposal(&id.to_string())
        .await
        .map_err(text)
        .and_then(|body| from_json::<RoutineReply>(&body))
    {
        Ok(reply) => HelpersMsg::Routines(RoutinesMsg::Accepted(reply)),
        Err(why) => unreached(&why).unwrap_or(HelpersMsg::Unreached),
    }
}

/// Turn the suggestion `id` down; nothing comes back unless the call failed.
pub(super) async fn decline(
    proxy: &CompanionHelpers2Proxy<'_>,
    id: &ProposalId,
) -> Option<HelpersMsg> {
    silent(proxy.decline_routine_proposal(&id.to_string()).await)
}
