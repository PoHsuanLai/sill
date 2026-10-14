//! Which arm the compositor's globals select.

use desktop_client::{DesktopBackend as Offered, ProtocolSet};
use sill_model::desktop::{DesktopBackend, GlobalOffer};

/// The arm for `offer`, from the globals alone (never a compositor's name or the environment):
/// what `desktop_client::backend_for` says the offer carries, with the ext list alone read as
/// the standard arm. Nothing offered is `Bare`, an empty desktop.
pub fn backend_of(offer: &GlobalOffer) -> DesktopBackend {
    let protocols = ProtocolSet::from_interfaces(offer.names());
    match desktop_client::backend_for(&protocols) {
        Some(Offered::Quire) => DesktopBackend::Quire,
        Some(Offered::Wlr | Offered::Ext) => DesktopBackend::Wlr,
        None => DesktopBackend::Bare,
    }
}
