//! porter-client's guest rows and asks, as questions; the verdict as its answer.

use porter_tailnet::{GuestAnswer, GuestRow};
use porter_tailnet::{GuestAsk, RowState};
use porter_tailscale::NodeId;
use sill_model::guest_asks::{Ask, Computer, Verdict};

/// inferd's well-known name.
pub(crate) const INFERD: &str = porter_dbus::names::INFERENCE_BUS;

/// The question a `GuestAsks` signal carries. A computer without a name is not one the person
/// can answer, so it says nothing.
pub(crate) fn asked(node: &NodeId, ask: GuestAsk) -> Option<Ask> {
    (!ask.name.is_empty()).then(|| Ask {
        node: Computer::new(node.as_str()),
        name: ask.name,
    })
}

/// The questions waiting in a `Guests` listing: the rows still asking.
pub(crate) fn waiting(rows: Vec<GuestRow>) -> Vec<Ask> {
    rows.into_iter()
        .filter(|row| row.state == RowState::Asking)
        .filter_map(|row| asked(&row.node, GuestAsk::new(row.name, row.since)))
        .collect()
}

/// The answer porter-client takes for `verdict`.
pub(crate) fn answer(verdict: Verdict) -> GuestAnswer {
    match verdict {
        Verdict::Allow => GuestAnswer::Allow,
        Verdict::Deny => GuestAnswer::Deny,
    }
}
