//! The status tray: an SNI host with dbusmenu (system-tray 0.8.9: SNI host + dbusmenu; registers a
//! watcher if none).
//!
//! [`TrayService`] and [`host`] do the effect (a `system_tray::client::Client` on the session bus);
//! [`icon`] turns a resolved [`TrayIcon`] into the art the bar draws (PNG bytes or a theme
//! file). Neither reaches `dioxus`, `ds` or `shell-host` (`scripts/check-boundary.sh`): the bar
//! reads this module only through [`Tray`], [`TrayCommand`] and [`icon_art`], via
//! `use_service::<TrayService>`.

mod host;
mod icon;
mod model;

pub use icon::{IconArt, icon_art};
pub use model::{
    Availability, Check, Pixmap, ScrollAxis, ScrollDelta, Tray, TrayCommand, TrayEvent, TrayIcon,
    TrayItem, TrayItemId, TrayMenuEntry, TrayMenuKind, TrayStatus, step,
};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// The tray host as a [`Service`], registering a `StatusNotifierWatcher` when none is on the bus
/// (`system_tray::client::Client::new`'s own `StatusNotifierWatcher::attach_to`: it tries to own
/// `org.kde.StatusNotifierWatcher` and quietly defers when a desktop already does). Without a
/// session bus the state stays at `Tray::default()`.
#[derive(Debug)]
pub enum TrayService {}

impl Service for TrayService {
    const NAME: &'static str = "tray";
    type State = Tray;
    type Command = TrayCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        async move {
            host::run(bus, io.state, io.commands).await;
            Ok(())
        }
    }
}
