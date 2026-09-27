//! The idle thread's protocol events.

use cosmic_client_toolkit::sctk::reexports::protocols_wlr::output_power_management::v1::client::{
    zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1,
    zwlr_output_power_v1::{self, ZwlrOutputPowerV1},
};
use cosmic_client_toolkit::wayland_client::globals::GlobalListContents;
use cosmic_client_toolkit::wayland_client::protocol::{
    wl_output::WlOutput, wl_registry, wl_registry::WlRegistry, wl_seat::WlSeat,
};
use cosmic_client_toolkit::wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use cosmic_client_toolkit::wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
    ext_idle_notifier_v1::ExtIdleNotifierV1,
};

use super::wl_state::WlState;

/// Outputs come and go after the first roundtrip.
impl Dispatch<WlRegistry, GlobalListContents> for WlState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } if interface == WlOutput::interface().name => {
                let output = registry.bind::<WlOutput, _, _>(name, version.min(4), qh, ());
                state.add_screen(name, output);
            }
            wl_registry::Event::GlobalRemove { name } => state.remove_screen(name),
            _ => {}
        }
    }
}

impl Dispatch<ExtIdleNotificationV1, u64> for WlState {
    fn event(
        state: &mut Self,
        _: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        id: &u64,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => state.idled(*id),
            ext_idle_notification_v1::Event::Resumed => state.resumed(*id),
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputPowerV1, ()> for WlState {
    fn event(
        state: &mut Self,
        control: &ZwlrOutputPowerV1,
        event: zwlr_output_power_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_power_v1::Event::Failed => state.power_failed(control),
            zwlr_output_power_v1::Event::Mode { .. } => {}
            _ => {}
        }
    }
}

/// Globals whose events idle does not follow.
macro_rules! ignore_events {
    ($($proxy:ty),*) => {$(
        impl Dispatch<$proxy, ()> for WlState {
            fn event(
                _: &mut Self,
                _: &$proxy,
                _: <$proxy as Proxy>::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    )*};
}

ignore_events!(
    ExtIdleNotifierV1,
    ZwlrOutputPowerManagerV1,
    WlSeat,
    WlOutput
);
