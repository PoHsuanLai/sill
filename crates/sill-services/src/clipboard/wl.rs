//! The data-control protocols the clipboard service binds, named here so the freeze proves both
//! build on the workspace's pinned crates (`wayland-protocols` 0.32 `staging` for `ext`, and
//! sctk's re-export of `wayland-protocols-wlr` for `zwlr`): no new dependency. Lane b adds the
//! dispatch (its own `wayland_client` connection and queue, as `cosmic_wl` has).

use cosmic_client_toolkit::sctk::reexports::protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;
use cosmic_client_toolkit::wayland_client::Proxy;
use cosmic_client_toolkit::wayland_protocols::ext::data_control::v1::client::ext_data_control_manager_v1::ExtDataControlManagerV1;

/// The globals the service looks for, preferred first.
pub(super) fn manager_interfaces() -> [&'static str; 2] {
    [
        ExtDataControlManagerV1::interface().name,
        ZwlrDataControlManagerV1::interface().name,
    ]
}

#[cfg(test)]
mod tests {
    use super::manager_interfaces;

    #[test]
    fn both_managers_are_known_by_their_global_names() {
        assert_eq!(
            manager_interfaces(),
            ["ext_data_control_manager_v1", "zwlr_data_control_manager_v1"]
        );
    }
}
