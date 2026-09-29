//! The protocol events, handed to [`WireState`]. The `ext` and `zwlr` families carry the same
//! events under different types, so one macro writes both families' impls.

use cosmic_client_toolkit::sctk::reexports::protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1 as wlr_device, zwlr_data_control_manager_v1 as wlr_manager,
    zwlr_data_control_offer_v1 as wlr_offer, zwlr_data_control_source_v1 as wlr_source,
};
use cosmic_client_toolkit::wayland_client::globals::GlobalListContents;
use cosmic_client_toolkit::wayland_client::protocol::wl_registry::WlRegistry;
use cosmic_client_toolkit::wayland_client::protocol::wl_seat::WlSeat;
use cosmic_client_toolkit::wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, event_created_child,
};
use cosmic_client_toolkit::wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1 as ext_device, ext_data_control_manager_v1 as ext_manager,
    ext_data_control_offer_v1 as ext_offer, ext_data_control_source_v1 as ext_source,
};

use super::wire_state::WireState;
use super::wl::{Offer, Source};

impl Dispatch<WlRegistry, GlobalListContents> for WireState {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for WireState {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: <WlSeat as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

/// The four impls of one family: its manager (no events), device, offer and source.
macro_rules! family {
    (
        $variant:ident,
        $manager:ty,
        $device_mod:ident :: $device:ident,
        $offer_mod:ident :: $offer:ident,
        $source_mod:ident :: $source:ident
    ) => {
        impl Dispatch<$manager, ()> for WireState {
            fn event(
                _: &mut Self,
                _: &$manager,
                _: <$manager as Proxy>::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }

        impl Dispatch<$device_mod::$device, ()> for WireState {
            fn event(
                state: &mut Self,
                _: &$device_mod::$device,
                event: $device_mod::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                match event {
                    $device_mod::Event::DataOffer { id } => state.offered(&Offer::$variant(id)),
                    $device_mod::Event::Selection { id } => {
                        state.selection(id.map(Offer::$variant))
                    }
                    $device_mod::Event::PrimarySelection { id } => {
                        state.primary_selection(id.map(Offer::$variant))
                    }
                    $device_mod::Event::Finished => state.finished(),
                    _ => {}
                }
            }

            event_created_child!(WireState, $device_mod::$device, [
                $device_mod::EVT_DATA_OFFER_OPCODE => ($offer_mod::$offer, ()),
            ]);
        }

        impl Dispatch<$offer_mod::$offer, ()> for WireState {
            fn event(
                state: &mut Self,
                offer: &$offer_mod::$offer,
                event: $offer_mod::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                if let $offer_mod::Event::Offer { mime_type } = event {
                    state.offer_type(&offer.id(), mime_type);
                }
            }
        }

        impl Dispatch<$source_mod::$source, ()> for WireState {
            fn event(
                state: &mut Self,
                source: &$source_mod::$source,
                event: $source_mod::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                match event {
                    $source_mod::Event::Send { fd, .. } => state.send(&source.id(), fd),
                    $source_mod::Event::Cancelled => {
                        state.cancelled(&Source::$variant(source.clone()))
                    }
                    _ => {}
                }
            }
        }
    };
}

family!(
    Ext,
    ext_manager::ExtDataControlManagerV1,
    ext_device::ExtDataControlDeviceV1,
    ext_offer::ExtDataControlOfferV1,
    ext_source::ExtDataControlSourceV1
);

family!(
    Wlr,
    wlr_manager::ZwlrDataControlManagerV1,
    wlr_device::ZwlrDataControlDeviceV1,
    wlr_offer::ZwlrDataControlOfferV1,
    wlr_source::ZwlrDataControlSourceV1
);
