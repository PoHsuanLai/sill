//! power-profiles-daemon over the system bus: seeds the three properties, then feeds every
//! change through [`super::reduce_power_profiles`], and carries out [`ProfileCommand`]s.

use std::collections::HashMap;
use std::pin::Pin;

use tokio::sync::{mpsc, watch};
use zbus::Connection;
use zbus::export::futures_core::Stream;
use zbus::fdo::DBusProxy;
use zbus::proxy::PropertyStream;
use zbus::zvariant::OwnedValue;

use super::ProfileCommand;
use super::model::{
    PowerProfiles, Profile, ProfileBus, ProfileDegraded, ProfileEvent, reduce_power_profiles,
};
use super::ppd::{HadessPowerProfilesProxy, UPowerPowerProfilesProxy};

/// Whichever of the two bus names answered, holding its own proxy: same properties, a
/// different interface name per bus (module doc), so one enum stands in for "the daemon" once
/// [`connect`] has found it.
enum Ppd<'a> {
    UPower(UPowerPowerProfilesProxy<'a>),
    Hadess(HadessPowerProfilesProxy<'a>),
}

impl Ppd<'_> {
    async fn active_profile(&self) -> zbus::Result<String> {
        match self {
            Ppd::UPower(p) => p.active_profile().await,
            Ppd::Hadess(p) => p.active_profile().await,
        }
    }

    async fn set_active_profile(&self, value: &str) -> zbus::Result<()> {
        match self {
            Ppd::UPower(p) => p.set_active_profile(value).await,
            Ppd::Hadess(p) => p.set_active_profile(value).await,
        }
    }

    async fn profiles(&self) -> zbus::Result<Vec<HashMap<String, OwnedValue>>> {
        match self {
            Ppd::UPower(p) => p.profiles().await,
            Ppd::Hadess(p) => p.profiles().await,
        }
    }

    async fn performance_degraded(&self) -> zbus::Result<String> {
        match self {
            Ppd::UPower(p) => p.performance_degraded().await,
            Ppd::Hadess(p) => p.performance_degraded().await,
        }
    }

    async fn receive_active_profile_changed(&self) -> PropertyStream<'_, String> {
        match self {
            Ppd::UPower(p) => p.receive_active_profile_changed().await,
            Ppd::Hadess(p) => p.receive_active_profile_changed().await,
        }
    }

    async fn receive_profiles_changed(
        &self,
    ) -> PropertyStream<'_, Vec<HashMap<String, OwnedValue>>> {
        match self {
            Ppd::UPower(p) => p.receive_profiles_changed().await,
            Ppd::Hadess(p) => p.receive_profiles_changed().await,
        }
    }

    async fn receive_performance_degraded_changed(&self) -> PropertyStream<'_, String> {
        match self {
            Ppd::UPower(p) => p.receive_performance_degraded_changed().await,
            Ppd::Hadess(p) => p.receive_performance_degraded_changed().await,
        }
    }
}

/// Runs until `commands` closes. Without a bus or the daemon, `tx` stays at its default.
pub(super) async fn run(
    tx: watch::Sender<PowerProfiles>,
    mut commands: mpsc::UnboundedReceiver<ProfileCommand>,
) {
    let Ok(connection) = Connection::system().await else {
        return;
    };
    let Some((bus, ppd)) = connect(&connection).await else {
        return;
    };
    seed(&tx, bus, &ppd).await;

    let mut active = ppd.receive_active_profile_changed().await;
    let mut profiles = ppd.receive_profiles_changed().await;
    let mut degraded = ppd.receive_performance_degraded_changed().await;
    let mut departures = watch_departure(&connection, bus).await;

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
                Some(ProfileCommand::Refresh) => seed(&tx, bus, &ppd).await,
                None => break,
            }
        }
    }
}

/// Tries `org.freedesktop.UPower.PowerProfiles` first (FINDINGS "M5 freeze": Fedora's), then
/// `net.hadess.PowerProfiles`. Building a proxy never itself touches the bus, so each candidate
/// is confirmed with a real property read before it is trusted.
async fn connect(connection: &Connection) -> Option<(ProfileBus, Ppd<'_>)> {
    if let Ok(proxy) = UPowerPowerProfilesProxy::new(connection).await
        && proxy.active_profile().await.is_ok()
    {
        return Some((ProfileBus::UPower, Ppd::UPower(proxy)));
    }
    if let Ok(proxy) = HadessPowerProfilesProxy::new(connection).await
        && proxy.active_profile().await.is_ok()
    {
        return Some((ProfileBus::Hadess, Ppd::Hadess(proxy)));
    }
    None
}

/// A stream of `NameOwnerChanged` for `bus`'s own well-known name only, so its `new_owner()`
/// being empty means the daemon itself left (not some unrelated name on the bus). `None` if the
/// `DBusProxy` or the match itself could not be built, which is no worse than not detecting the
/// departure at all ([`next_opt`] simply never fires for it).
async fn watch_departure(
    connection: &Connection,
    bus: ProfileBus,
) -> Option<zbus::fdo::NameOwnerChangedStream> {
    let name = bus_name(bus);
    let dbus = DBusProxy::new(connection).await.ok()?;
    dbus.receive_name_owner_changed_with_args(&[(0, name)])
        .await
        .ok()
}

fn bus_name(bus: ProfileBus) -> &'static str {
    match bus {
        ProfileBus::UPower => "org.freedesktop.UPower.PowerProfiles",
        ProfileBus::Hadess => "net.hadess.PowerProfiles",
    }
}

/// Reads every property once and publishes the resulting `PowerProfiles`.
async fn seed(tx: &watch::Sender<PowerProfiles>, bus: ProfileBus, ppd: &Ppd<'_>) {
    apply(tx, ProfileEvent::Found(bus));
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
    let next = reduce_power_profiles(tx.borrow().clone(), event);
    let _ = tx.send(next);
}

/// Polls one `Unpin` stream for its next item, for use as a `tokio::select!` branch.
async fn next<S>(stream: &mut S) -> Option<S::Item>
where
    S: Stream + Unpin,
{
    std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}

/// [`next`] for a stream that might not exist: `None` never resolves, so the `tokio::select!`
/// branch it backs simply never fires instead of needing its own guard.
async fn next_opt<S>(stream: &mut Option<S>) -> Option<S::Item>
where
    S: Stream + Unpin,
{
    match stream {
        Some(stream) => next(stream).await,
        None => std::future::pending().await,
    }
}
