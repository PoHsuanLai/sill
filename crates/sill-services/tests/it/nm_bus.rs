//! The network and Wi-Fi services against a stand-in `org.freedesktop.NetworkManager` on a
//! private bus playing the system bus, never the real one. The stand-in serves the members the
//! airwaves-nm proxies call and records the calls. The tests need a `dbus-daemon` and skip
//! without one.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use sill_model::network::{Connectivity, Link, Network, NetworkCommand};
use sill_model::units::{Percent, Presence};
use sill_model::wifi::{Association, Radio, Security, Ssid, Wifi, WifiCommand};
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::network::NetworkService;
use sill_services::service::spawn;
use sill_services::wifi::WifiService;
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;
use zbus::interface;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

const NM: &str = "/org/freedesktop/NetworkManager";
const DEVICE: &str = "/org/freedesktop/NetworkManager/Devices/1";
const ACTIVE: &str = "/org/freedesktop/NetworkManager/ActiveConnection/1";
const AP: &str = "/org/freedesktop/NetworkManager/AccessPoint/1";
const SETTINGS: &str = "/org/freedesktop/NetworkManager/Settings";
const SAVED: &str = "/org/freedesktop/NetworkManager/Settings/1";

/// What the stand-in holds and heard.
#[derive(Debug, Default)]
struct Held {
    wireless_enabled: bool,
    calls: Vec<String>,
}

type Shared = Arc<Mutex<Held>>;

fn lock(shared: &Shared) -> MutexGuard<'_, Held> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

fn path(text: &str) -> OwnedObjectPath {
    OwnedObjectPath::try_from(text).unwrap_or_else(|e| panic!("{e}"))
}

struct Manager(Shared);

#[interface(name = "org.freedesktop.NetworkManager")]
impl Manager {
    fn get_all_devices(&self) -> Vec<OwnedObjectPath> {
        vec![path(DEVICE)]
    }

    fn activate_connection(
        &self,
        connection: OwnedObjectPath,
        device: OwnedObjectPath,
        _specific_object: OwnedObjectPath,
    ) -> OwnedObjectPath {
        lock(&self.0)
            .calls
            .push(format!("ActivateConnection({connection}, {device})"));
        path(ACTIVE)
    }

    #[zbus(property)]
    fn primary_connection(&self) -> OwnedObjectPath {
        path(ACTIVE)
    }

    #[zbus(property)]
    fn activating_connection(&self) -> OwnedObjectPath {
        path("/")
    }

    #[zbus(property)]
    fn connectivity(&self) -> u32 {
        4
    }

    #[zbus(property)]
    fn wireless_enabled(&self) -> bool {
        lock(&self.0).wireless_enabled
    }

    #[zbus(property)]
    fn set_wireless_enabled(&mut self, value: bool) {
        lock(&self.0).wireless_enabled = value;
    }

    #[zbus(property)]
    fn wireless_hardware_enabled(&self) -> bool {
        true
    }
}

struct Active;

#[interface(name = "org.freedesktop.NetworkManager.Connection.Active")]
impl Active {
    #[zbus(property)]
    fn state(&self) -> u32 {
        2
    }

    #[zbus(property, name = "Type")]
    fn kind(&self) -> String {
        "802-11-wireless".to_owned()
    }

    #[zbus(property)]
    fn devices(&self) -> Vec<OwnedObjectPath> {
        vec![path(DEVICE)]
    }
}

struct Device(Shared);

#[interface(name = "org.freedesktop.NetworkManager.Device")]
impl Device {
    fn disconnect(&self) {
        lock(&self.0).calls.push("Disconnect".to_owned());
    }

    #[zbus(property)]
    fn device_type(&self) -> u32 {
        2
    }

    #[zbus(property)]
    fn state(&self) -> u32 {
        100
    }
}

struct Wireless(Shared);

#[interface(name = "org.freedesktop.NetworkManager.Device.Wireless")]
impl Wireless {
    fn request_scan(&self, _options: HashMap<String, OwnedValue>) {
        lock(&self.0).calls.push("RequestScan".to_owned());
    }

    #[zbus(property)]
    fn access_points(&self) -> Vec<OwnedObjectPath> {
        vec![path(AP)]
    }

    #[zbus(property)]
    fn active_access_point(&self) -> OwnedObjectPath {
        path(AP)
    }

    #[zbus(property)]
    fn last_scan(&self) -> i64 {
        1
    }
}

struct AccessPoint;

#[interface(name = "org.freedesktop.NetworkManager.AccessPoint")]
impl AccessPoint {
    #[zbus(property)]
    fn ssid(&self) -> Vec<u8> {
        b"home".to_vec()
    }

    #[zbus(property)]
    fn strength(&self) -> u8 {
        70
    }

    #[zbus(property)]
    fn flags(&self) -> u32 {
        1
    }

    #[zbus(property)]
    fn wpa_flags(&self) -> u32 {
        0
    }

    #[zbus(property)]
    fn rsn_flags(&self) -> u32 {
        0
    }
}

struct Settings;

#[interface(name = "org.freedesktop.NetworkManager.Settings")]
impl Settings {
    #[zbus(property)]
    fn connections(&self) -> Vec<OwnedObjectPath> {
        vec![path(SAVED)]
    }
}

struct Saved;

#[interface(name = "org.freedesktop.NetworkManager.Settings.Connection")]
impl Saved {
    fn get_settings(&self) -> HashMap<String, HashMap<String, OwnedValue>> {
        let ssid =
            OwnedValue::try_from(Value::from(b"home".to_vec())).unwrap_or_else(|e| panic!("{e}"));
        HashMap::from([(
            "802-11-wireless".to_owned(),
            HashMap::from([("ssid".to_owned(), ssid)]),
        )])
    }
}

/// The stand-in on `bus`, serving as `org.freedesktop.NetworkManager`.
async fn serve(bus: &PrivateBus) -> (zbus::Connection, Shared) {
    let shared: Shared = Arc::new(Mutex::new(Held {
        wireless_enabled: true,
        calls: Vec::new(),
    }));
    let connection = zbus::connection::Builder::address(bus.address.as_str())
        .and_then(|b| b.name("org.freedesktop.NetworkManager"))
        .and_then(|b| b.serve_at(NM, Manager(shared.clone())))
        .and_then(|b| b.serve_at(ACTIVE, Active))
        .and_then(|b| b.serve_at(DEVICE, Device(shared.clone())))
        .and_then(|b| b.serve_at(DEVICE, Wireless(shared.clone())))
        .and_then(|b| b.serve_at(AP, AccessPoint))
        .and_then(|b| b.serve_at(SETTINGS, Settings))
        .and_then(|b| b.serve_at(SAVED, Saved));
    let connection = connection.unwrap_or_else(|e| panic!("{e}"));
    let connection = connection
        .build()
        .await
        .unwrap_or_else(|e| panic!("the stand-in NetworkManager: {e}"));
    (connection, shared)
}

fn env_for(bus: &PrivateBus, scratch: &TempDir) -> Env {
    Env {
        buses: Buses::new(BusRoute::Address(bus.address.clone()), BusRoute::Absent),
        ..Env::isolated(scratch.path())
    }
}

async fn until<T>(rx: &mut watch::Receiver<T>, what: &str, ok: impl FnMut(&T) -> bool) {
    let waited = tokio::time::timeout(Duration::from_secs(10), rx.wait_for(ok)).await;
    assert!(matches!(waited, Ok(Ok(_))), "timed out waiting for {what}");
}

async fn calls_include(shared: &Shared, wanted: &str) {
    for _ in 0..100 {
        if lock(shared).calls.iter().any(|c| c == wanted) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("never heard {wanted}: {:?}", lock(shared).calls);
}

#[test]
fn the_network_follows_the_primary_wifi_connection() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("nm-network");
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    rt.block_on(async {
        let (_nm, _shared) = serve(&bus).await;
        let env = env_for(&bus, &scratch);
        let handle = spawn::<NetworkService>(&tokio::runtime::Handle::current(), &env, ());
        let mut rx = handle.state;
        until(&mut rx, "the Wi-Fi link", |n: &Network| {
            n.link
                == Link::Wifi {
                    ssid: "home".to_owned(),
                    strength: Percent(70),
                }
                && n.connectivity == Connectivity::from_nm(4)
        })
        .await;
        let _ = handle.control.send(NetworkCommand::Refresh);
        drop(handle.control);
    });
}

#[test]
fn wifi_reads_the_device_and_carries_out_commands() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("nm-wifi");
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    rt.block_on(async {
        let (_nm, shared) = serve(&bus).await;
        let env = env_for(&bus, &scratch);
        let handle = spawn::<WifiService>(&tokio::runtime::Handle::current(), &env, ());
        let mut rx = handle.state;
        until(&mut rx, "the access point and saved network", |w: &Wifi| {
            w.device == Presence::Present
                && w.radio == Radio::On
                && w.saved == [Ssid("home".to_owned())]
                && w.access_points.len() == 1
                && w.access_points[0].security == Security::Secured
                && w.access_points[0].association == Association::Connected
                && w.access_points[0].strength == Percent(70)
        })
        .await;

        let send = |command| {
            handle
                .control
                .send(command)
                .unwrap_or_else(|e| panic!("{e}"))
        };
        send(WifiCommand::Scan);
        calls_include(&shared, "RequestScan").await;
        send(WifiCommand::Connect(Ssid("home".to_owned())));
        calls_include(&shared, &format!("ActivateConnection({SAVED}, {DEVICE})")).await;
        send(WifiCommand::Disconnect);
        calls_include(&shared, "Disconnect").await;
        send(WifiCommand::Radio(Radio::Off));
        for _ in 0..100 {
            if !lock(&shared).wireless_enabled {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("WirelessEnabled was never set to false");
    });
}

#[test]
fn wifi_follows_networkmanager_that_starts_after_the_shell() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("nm-wifi-late");
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    rt.block_on(async {
        let env = env_for(&bus, &scratch);
        let handle = spawn::<WifiService>(&tokio::runtime::Handle::current(), &env, ());
        let mut rx = handle.state;
        // No NetworkManager yet: the service waits instead of ending.
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(rx.borrow().device, Presence::Absent);

        let (_nm, _shared) = serve(&bus).await;
        until(
            &mut rx,
            "the device once NetworkManager is up",
            |w: &Wifi| w.device == Presence::Present && w.access_points.len() == 1,
        )
        .await;
    });
}
