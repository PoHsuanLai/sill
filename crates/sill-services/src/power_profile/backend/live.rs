//! power-profiles-daemon over the system bus: seeds the three properties, then feeds every
//! change through [`sill_model::power_profile::step`], and carries out [`ProfileCommand`]s.

use std::collections::HashMap;

use sill_model::power_profile::{
    PowerProfiles, Profile, ProfileCommand, ProfileDegraded, ProfileEvent, step,
};
use tokio::sync::{mpsc, watch};
use zbus::Connection;
use zbus::zvariant::OwnedValue;

use super::ppd::UPowerPowerProfilesProxy;
use crate::env::Bus;
use crate::error::ServiceError;
use crate::owner::{self, Ended};
use crate::stream::next;

const SERVICE: &str = "org.freedesktop.UPower.PowerProfiles";

/// Runs until `commands` closes. Without the daemon `tx` stays at its default; the daemon
/// starting late, leaving or restarting is followed through `NameOwnerChanged` ([`crate::owner`]).
pub(crate) async fn run(
    bus: Bus,
    tx: watch::Sender<PowerProfiles>,
    mut commands: mpsc::UnboundedReceiver<ProfileCommand>,
) -> Result<(), ServiceError> {
    let connection = bus.connect_waiting().await?;
    owner::follow_owner(
        &connection,
        SERVICE,
        &mut commands,
        async |commands| session(&connection, &tx, commands).await,
        |_| apply(&tx, ProfileEvent::Gone),
    )
    .await;
    Ok(())
}

/// One daemon lifetime: seed the three properties, then follow every change and carry out
/// [`ProfileCommand`]s.
async fn session(
    connection: &Connection,
    tx: &watch::Sender<PowerProfiles>,
    commands: &mut mpsc::UnboundedReceiver<ProfileCommand>,
) -> Ended {
    let Some(ppd) = connect(connection).await else {
        return Ended::Unreadable;
    };
    seed(tx, &ppd).await;

    let mut active = ppd.receive_active_profile_changed().await;
    let mut profiles = ppd.receive_profiles_changed().await;
    let mut degraded = ppd.receive_performance_degraded_changed().await;

    loop {
        tokio::select! {
            Some(change) = next(&mut active) => {
                if let Ok(value) = change.get().await
                    && let Some(profile) = Profile::from_daemon(&value)
                {
                    apply(tx, ProfileEvent::Active(profile));
                }
            }
            Some(change) = next(&mut profiles) => {
                if let Ok(value) = change.get().await {
                    apply(tx, ProfileEvent::Offered(value.iter().filter_map(profile_of).collect()));
                }
            }
            Some(change) = next(&mut degraded) => {
                if let Ok(value) = change.get().await {
                    apply(tx, ProfileEvent::Degraded(degraded_of(value)));
                }
            }
            command = commands.recv() => match command {
                Some(ProfileCommand::Set(profile)) => {
                    if tx.borrow().offered.contains(&profile) {
                        let _ = ppd.set_active_profile(profile.daemon_word()).await;
                    }
                }
                Some(ProfileCommand::Refresh) => seed(tx, &ppd).await,
                None => return Ended::Commands,
            }
        }
    }
}

/// The daemon's proxy, confirmed with a real property read (building a proxy never itself
/// touches the bus).
async fn connect(connection: &Connection) -> Option<UPowerPowerProfilesProxy<'_>> {
    let proxy = UPowerPowerProfilesProxy::new(connection).await.ok()?;
    proxy.active_profile().await.ok().map(|_| proxy)
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
