//! What the compositor offers, read from its registry or from the wire client's bound set.

use cosmic_client_toolkit::wayland_client::globals::GlobalList;
use desktop_client::ProtocolSet;
use sill_model::desktop::GlobalOffer;

/// The globals the registry listed, with their versions.
pub fn offer_of_registry(globals: &GlobalList) -> GlobalOffer {
    globals.contents().with_list(|list| {
        GlobalOffer::from_globals(list.iter().map(|g| (g.interface.as_str(), g.version)))
    })
}

/// The globals the wire client found among the ones it uses (their versions are not kept, so
/// each is listed at 1).
pub fn offer_of_protocols(protocols: &ProtocolSet) -> GlobalOffer {
    GlobalOffer::from_globals(protocols.iter().map(|p| (p.interface(), 1)))
}
