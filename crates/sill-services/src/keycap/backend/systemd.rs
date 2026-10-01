//! What systemd says about keycap's unit, and the restart the person can ask for. Both go over
//! the system bus (polkit lets the active session restart `keycap.service`).

use sill_model::keycap::{UnitState, unit_state};
use zbus::proxy::CacheProperties;
use zbus::zvariant::OwnedObjectPath;

use crate::env::Bus;

/// The unit keycap runs as.
const UNIT: &str = "keycap.service";

/// The error name `GetUnit` answers with for a unit the manager has not loaded.
const NO_SUCH_UNIT: &str = "org.freedesktop.systemd1.NoSuchUnit";

#[zbus::proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
trait Manager {
    fn get_unit(&self, name: &str) -> zbus::Result<OwnedObjectPath>;
    fn reset_failed_unit(&self, name: &str) -> zbus::Result<()>;
    fn restart_unit(&self, name: &str, mode: &str) -> zbus::Result<OwnedObjectPath>;
}

#[zbus::proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1"
)]
trait Unit {
    #[zbus(property)]
    fn active_state(&self) -> zbus::Result<String>;
}

/// The unit's state; `None` when the bus or the manager could not say, so what is known stays.
pub(crate) async fn read_state(bus: &Bus) -> Option<UnitState> {
    let connection = bus.connect().await.ok()??;
    let manager = ManagerProxy::new(&connection).await.ok()?;
    match manager.get_unit(UNIT).await {
        Ok(path) => {
            let unit = UnitProxy::builder(&connection)
                .path(path)
                .ok()?
                .cache_properties(CacheProperties::No)
                .build()
                .await
                .ok()?;
            unit_state(&unit.active_state().await.ok()?)
        }
        Err(zbus::Error::MethodError(name, _, _)) if name.as_str() == NO_SUCH_UNIT => {
            Some(UnitState::NotFound)
        }
        Err(_) => None,
    }
}

/// Clear the unit's failure and start it again.
pub(crate) async fn restart(bus: &Bus) -> zbus::Result<()> {
    let connection = bus
        .connect()
        .await?
        .ok_or_else(|| zbus::Error::Failure("there is no system bus".to_owned()))?;
    let manager = ManagerProxy::new(&connection).await?;
    // A unit that is not failed has nothing to reset; the restart is what matters.
    let _ = manager.reset_failed_unit(UNIT).await;
    manager.restart_unit(UNIT, "replace").await.map(drop)
}
