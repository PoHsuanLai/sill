//! The data-control protocols the clipboard service binds (`wayland-protocols` 0.32 `staging`
//! for `ext`, and sctk's re-export of `wayland-protocols-wlr` for `zwlr`; no new dependency),
//! and one face over both: the two are the same protocol under two names, so the live loop
//! speaks to a [`Manager`], a [`Device`], an [`Offer`] and a [`Source`] and never to either
//! family directly.

use std::os::fd::BorrowedFd;

use smithay_client_toolkit::reexports::protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::ZwlrDataControlDeviceV1;
use smithay_client_toolkit::reexports::protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;
use smithay_client_toolkit::reexports::protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::ZwlrDataControlOfferV1;
use smithay_client_toolkit::reexports::protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::ZwlrDataControlSourceV1;
use wayland_client::{Dispatch, Proxy, QueueHandle};
use wayland_client::backend::ObjectId;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols::ext::data_control::v1::client::ext_data_control_device_v1::ExtDataControlDeviceV1;
use wayland_protocols::ext::data_control::v1::client::ext_data_control_manager_v1::ExtDataControlManagerV1;
use wayland_protocols::ext::data_control::v1::client::ext_data_control_offer_v1::ExtDataControlOfferV1;
use wayland_protocols::ext::data_control::v1::client::ext_data_control_source_v1::ExtDataControlSourceV1;
use sill_model::clipboard::ClipProtocol;

/// The globals the service looks for, preferred first.
pub(crate) fn manager_interfaces() -> [&'static str; 2] {
    [
        ExtDataControlManagerV1::interface().name,
        ZwlrDataControlManagerV1::interface().name,
    ]
}

/// The bound manager.
#[derive(Debug)]
pub(crate) enum Manager {
    Ext(ExtDataControlManagerV1),
    Wlr(ZwlrDataControlManagerV1),
}

/// The seat's data-control device.
#[derive(Debug)]
pub(crate) enum Device {
    Ext(ExtDataControlDeviceV1),
    Wlr(ZwlrDataControlDeviceV1),
}

/// A selection offered to the service.
#[derive(Debug, Clone)]
pub(crate) enum Offer {
    Ext(ExtDataControlOfferV1),
    Wlr(ZwlrDataControlOfferV1),
}

/// A selection the service offers.
#[derive(Debug, Clone)]
pub(crate) enum Source {
    Ext(ExtDataControlSourceV1),
    Wlr(ZwlrDataControlSourceV1),
}

/// `ext` if the compositor has it, else `zwlr` (version 1: it has no primary selection, which
/// the service never follows), else nothing.
pub(crate) fn bind_manager<S>(globals: &GlobalList, qh: &QueueHandle<S>) -> Option<Manager>
where
    S: Dispatch<ExtDataControlManagerV1, ()> + Dispatch<ZwlrDataControlManagerV1, ()> + 'static,
{
    globals
        .bind::<ExtDataControlManagerV1, _, _>(qh, 1..=1, ())
        .map(Manager::Ext)
        .or_else(|_| {
            globals
                .bind::<ZwlrDataControlManagerV1, _, _>(qh, 1..=1, ())
                .map(Manager::Wlr)
        })
        .ok()
}

impl Manager {
    pub(crate) fn protocol(&self) -> ClipProtocol {
        match self {
            Manager::Ext(_) => ClipProtocol::Ext,
            Manager::Wlr(_) => ClipProtocol::Wlr,
        }
    }

    pub(crate) fn device<S>(&self, seat: &WlSeat, qh: &QueueHandle<S>) -> Device
    where
        S: Dispatch<ExtDataControlDeviceV1, ()> + Dispatch<ZwlrDataControlDeviceV1, ()> + 'static,
    {
        match self {
            Manager::Ext(m) => Device::Ext(m.get_data_device(seat, qh, ())),
            Manager::Wlr(m) => Device::Wlr(m.get_data_device(seat, qh, ())),
        }
    }

    pub(crate) fn source<S>(&self, qh: &QueueHandle<S>) -> Source
    where
        S: Dispatch<ExtDataControlSourceV1, ()> + Dispatch<ZwlrDataControlSourceV1, ()> + 'static,
    {
        match self {
            Manager::Ext(m) => Source::Ext(m.create_data_source(qh, ())),
            Manager::Wlr(m) => Source::Wlr(m.create_data_source(qh, ())),
        }
    }
}

impl Device {
    /// Make `source` the selection (never the primary selection). A source of the other
    /// family cannot be made (one manager makes both), so it is ignored.
    pub(crate) fn set_selection(&self, source: &Source) {
        match (self, source) {
            (Device::Ext(d), Source::Ext(s)) => d.set_selection(Some(s)),
            (Device::Wlr(d), Source::Wlr(s)) => d.set_selection(Some(s)),
            (Device::Ext(_), Source::Wlr(_)) | (Device::Wlr(_), Source::Ext(_)) => {}
        }
    }
}

impl Offer {
    pub(crate) fn id(&self) -> ObjectId {
        match self {
            Offer::Ext(o) => o.id(),
            Offer::Wlr(o) => o.id(),
        }
    }

    /// Ask the source to write `mime` into `fd`.
    pub(crate) fn receive(&self, mime: String, fd: BorrowedFd<'_>) {
        match self {
            Offer::Ext(o) => o.receive(mime, fd),
            Offer::Wlr(o) => o.receive(mime, fd),
        }
    }

    pub(crate) fn destroy(&self) {
        match self {
            Offer::Ext(o) => o.destroy(),
            Offer::Wlr(o) => o.destroy(),
        }
    }
}

impl Source {
    pub(crate) fn id(&self) -> ObjectId {
        match self {
            Source::Ext(s) => s.id(),
            Source::Wlr(s) => s.id(),
        }
    }

    pub(crate) fn offer(&self, mime: &str) {
        match self {
            Source::Ext(s) => s.offer(mime.to_owned()),
            Source::Wlr(s) => s.offer(mime.to_owned()),
        }
    }

    pub(crate) fn destroy(&self) {
        match self {
            Source::Ext(s) => s.destroy(),
            Source::Wlr(s) => s.destroy(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::manager_interfaces;

    #[test]
    fn both_managers_are_known_by_their_global_names() {
        assert_eq!(
            manager_interfaces(),
            [
                "ext_data_control_manager_v1",
                "zwlr_data_control_manager_v1"
            ]
        );
    }
}
