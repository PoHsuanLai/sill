//! Which arm the compositor's globals select.

use desktop_client::{DesktopBackend as Offered, ProtocolSet};
use sill_model::desktop::{DesktopBackend, GlobalOffer};

/// COSMIC's own toplevel info: a compositor that lists it also lists the ext toplevel list,
/// which alone does not make it a standard-protocol compositor.
const COSMIC_TOPLEVELS: &str = "zcosmic_toplevel_info_v1";

/// The arm for `offer`, from the globals alone (never a compositor's name or the environment):
/// what `desktop_client::backend_for` says the offer carries, with the ext list alone read as
/// the standard arm unless COSMIC's own globals are there. Nothing offered is `Cosmic`, which
/// degrades to an empty desktop.
pub fn backend_of(offer: &GlobalOffer) -> DesktopBackend {
    let protocols = ProtocolSet::from_interfaces(offer.names());
    match desktop_client::backend_for(&protocols) {
        Some(Offered::Quire) => DesktopBackend::Quire,
        Some(Offered::Wlr) => DesktopBackend::Wlr,
        Some(Offered::Ext) if !offer.offers(COSMIC_TOPLEVELS) => DesktopBackend::Wlr,
        Some(_) | None => DesktopBackend::Cosmic,
    }
}
