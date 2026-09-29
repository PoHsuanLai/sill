//! power-profiles-daemon over the system bus: seeds the three properties, then feeds every
//! change through [`super::step`], and carries out [`ProfileCommand`]s.

use std::collections::HashMap;

use tokio::sync::{mpsc, watch};
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::zvariant::OwnedValue;

use super::ProfileCommand;
use super::model::{PowerProfiles, Profile, ProfileDegraded, ProfileEvent, step};
use super::ppd::UPowerPowerProfilesProxy;
use crate::env::Bus;
use crate::error::ServiceError;
use crate::stream::{next, next_opt};

/// Runs until `commands` closes. Without a bus or the daemon, `tx` stays at its default.
pub(super) async fn run(
    bus: Bus,
    tx: watch::Sender<PowerProfiles>,
    mut commands: mpsc::UnboundedReceiver<ProfileCommand>,
) -> Result<(), ServiceError> {
    let connection = bus.connected().await?;
    let Some(ppd) = connect(&connection).await else {
        return Ok(());
    };
    seed(&tx, &ppd).await;

    let mut active = ppd.receive_active_profile_changed().await;
    let mut profiles = ppd.receive_profiles_changed().await;
    let mut degraded = ppd.receive_performance_degraded_changed().await;
    let mut departures = watch_departure(&connection).await;

    loop {
        tokio::select! {
            Some(signal) = next_opt(&mut departures) => {
                if let Ok(args) = signal.args()
                    && args.new_owner().is_none()
                {
                    // The daemon left the bus; a later `Refresh` (or the next property change,
                    // once whoever now owns the name answers) re-seeds through the same proxy —
                    // destinations route by well-known name, so it needs no reconnect.
                    apply(&tx, ProfileEvent::Gone);
                }
            }
            Some(change) = next(&mut active) => {
                if let Ok(value) = change.get().await
                    && let Some(profile) = Profile::from_daemon(&value)
                {
                    apply(&tx, ProfileEvent::Active(profile));
                }
            }
            Some(change) = next(&mut profiles) => {
                if let Ok(value) = change.get().await {
                    apply(&tx, ProfileEvent::Offered(value.iter().filter_map(profile_of).collect()));
                }
            }
            Some(change) = next(&mut degraded) => {
                if let Ok(value) = change.get().await {
                    apply(&tx, ProfileEvent::Degraded(degraded_of(value)));
                }
            }
            command = commands.recv() => match command {
                Some(ProfileCommand::Set(profile)) => {
                    if tx.borrow().offered.contains(&profile) {
                        let _ = ppd.set_active_profile(profile.daemon_word()).await;
                    }
                }
                Some(ProfileCommand::Refresh) => seed(&tx, &ppd).await,
                None => break,
            }
        }
    }
    Ok(())
}

/// The daemon's proxy, confirmed with a real property read (building a proxy never itself
/// touches the bus).
async fn connect(connection: &Connection) -> Option<UPowerPowerProfilesProxy<'_>> {
    let proxy = UPowerPowerProfilesProxy::new(connection).await.ok()?;
    proxy.active_profile().await.ok().map(|_| proxy)
}

/// A stream of `NameOwnerChanged` for the daemon's own well-known name only, so its `new_owner()`
/// being empty means the daemon itself left (not some unrelated name on the bus). `None` if the
/// `DBusProxy` or the match itself could not be built, which is no worse than not detecting the
/// departure at all ([`next_opt`] simply never fires for it).
async fn watch_departure(connection: &Connection) -> Option<zbus::fdo::NameOwnerChangedStream> {
    let name = "org.freedesktop.UPower.PowerProfiles";
    let dbus = DBusProxy::new(connection).await.ok()?;
    dbus.receive_name_owner_changed_with_args(&[(0, name)])
        .await
        .ok()
}

/// Reads every property once and publishes the resulting `PowerProfiles`.
async fn seed(tx: &watch::Sender<PowerProfiles>, ppd: &UPowerPowerProfilesProxy<'_>) {
    apply(tx, ProfileEvent::Found);
    if let Ok(active) = ppd.active_profile().await
        && let Some(profile) = Profile::from_daemon(&active)
    {
        apply(tx, ProfileEvent::Active(profile));
    }
    if let Ok(offered) = ppd.profiles().await {
        apply(
            tx,
            ProfileEvent::Offered(offered.iter().filter_map(profile_of).collect()),
        );
    }
    if let Ok(degraded) = ppd.performance_degraded().await {
        apply(tx, ProfileEvent::Degraded(degraded_of(degraded)));
    }
}

/// One entry of `Profiles` (`aa{sv}`, each a dict with at least a `Profile` key) to the
/// [`Profile`] it names, or `None` for a driver-specific entry this shell has no button for.
fn profile_of(entry: &HashMap<String, OwnedValue>) -> Option<Profile> {
    let word = entry.get("Profile")?;
    let word = String::try_from(word.clone()).ok()?;
    Profile::from_daemon(&word)
}

/// `PerformanceDegraded`'s empty string is "not degraded"; any other word is the reason.
fn degraded_of(reason: String) -> ProfileDegraded {
    if reason.is_empty() {
        ProfileDegraded::No
    } else {
        ProfileDegraded::Because(reason)
    }
}

/// Reduces `event` onto the receiver's current value and publishes the result.
fn apply(tx: &watch::Sender<PowerProfiles>, event: ProfileEvent) {
    let next = step(tx.borrow().clone(), event);
    let _ = tx.send(next);
}
