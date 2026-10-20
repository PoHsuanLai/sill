//! The battery, power profile and keyboard backlight services against stand-ins for UPower and
//! power-profiles-daemon on a private bus playing the system bus, never the real one. The tests
//! need a `dbus-daemon` and skip without one.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use sill_model::battery::{Battery, Charge};
use sill_model::kbd_backlight::{KbdBacklight, KbdCommand};
use sill_model::power_profile::{PowerProfiles, Profile, ProfileDegraded};
use sill_model::units::{Percent, Presence, Seconds};
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::service::spawn;
use sill_services::{BatteryService, KbdBacklightService, PowerProfileService};
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;
use zbus::interface;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

const UPOWER: &str = "/org/freedesktop/UPower";
const DISPLAY: &str = "/org/freedesktop/UPower/devices/DisplayDevice";
const KBD: &str = "/org/freedesktop/UPower/KbdBacklight";
const PROFILES: &str = "/org/freedesktop/UPower/PowerProfiles";

/// What the stand-ins heard.
#[derive(Debug, Default)]
struct Heard {
    kbd_steps: Vec<i32>,
}

type Shared = Arc<Mutex<Heard>>;

fn lock(shared: &Shared) -> MutexGuard<'_, Heard> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

struct Manager;

#[interface(name = "org.freedesktop.UPower")]
impl Manager {
    fn get_display_device(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from(DISPLAY).unwrap_or_else(|e| panic!("{e}"))
    }
}

struct Device;

#[interface(name = "org.freedesktop.UPower.Device")]
impl Device {
    fn refresh(&self) {}

    #[zbus(property)]
    fn is_present(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn percentage(&self) -> f64 {
        64.4
    }

    #[zbus(property)]
    fn state(&self) -> u32 {
        2
    }

    #[zbus(property)]
    fn time_to_empty(&self) -> i64 {
        7200
    }

    #[zbus(property)]
    fn time_to_full(&self) -> i64 {
        0
    }
}

struct Keyboard(Shared);

#[interface(name = "org.freedesktop.UPower.KbdBacklight")]
impl Keyboard {
    fn get_brightness(&self) -> i32 {
        1
    }

    fn get_max_brightness(&self) -> i32 {
        3
    }

    fn set_brightness(&self, value: i32) {
        lock(&self.0).kbd_steps.push(value);
    }
}

struct Profiles;

#[interface(name = "org.freedesktop.UPower.PowerProfiles")]
impl Profiles {
    #[zbus(property)]
    fn active_profile(&self) -> String {
        "balanced".to_owned()
    }

    #[zbus(property)]
    fn set_active_profile(&mut self, _value: String) {}

    #[zbus(property)]
    fn profiles(&self) -> Vec<HashMap<String, OwnedValue>> {
        ["power-saver", "balanced"]
            .into_iter()
            .map(|word| {
                let word =
                    OwnedValue::try_from(Value::from(word)).unwrap_or_else(|e| panic!("{e}"));
                HashMap::from([("Profile".to_owned(), word)])
            })
            .collect()
    }

    #[zbus(property)]
    fn performance_degraded(&self) -> String {
        String::new()
    }
}

/// The stand-ins on `bus`: one connection owning both `org.freedesktop.UPower` and the profile
/// daemon's name, as the real pair does on a laptop.
async fn serve(bus: &PrivateBus) -> (zbus::Connection, Shared) {
    let shared: Shared = Arc::default();
    let built = zbus::connection::Builder::address(bus.address.as_str())
        .and_then(|b| b.name("org.freedesktop.UPower"))
        .and_then(|b| b.name("org.freedesktop.UPower.PowerProfiles"))
        .and_then(|b| b.serve_at(UPOWER, Manager))
        .and_then(|b| b.serve_at(DISPLAY, Device))
        .and_then(|b| b.serve_at(KBD, Keyboard(shared.clone())))
        .and_then(|b| b.serve_at(PROFILES, Profiles));
    let connection = built
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("the stand-in UPower: {e}"));
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

#[test]
fn the_three_follow_a_upower_that_starts_late_and_leaves() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("upower");
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    rt.block_on(async {
        let env = env_for(&bus, &scratch);
        let here = tokio::runtime::Handle::current();
        let battery = spawn::<BatteryService>(&here, &env, ());
        let profiles = spawn::<PowerProfileService>(&here, &env, ());
        let kbd = spawn::<KbdBacklightService>(&here, &env, ());
        let (mut battery_rx, mut profiles_rx, mut kbd_rx) =
            (battery.state, profiles.state, kbd.state);
        // Nothing is on the bus yet: the services wait instead of ending.
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(battery_rx.borrow().presence, Presence::Absent);

        let (upower, shared) = serve(&bus).await;
        until(&mut battery_rx, "the battery", |b: &Battery| {
            b.presence == Presence::Present
                && b.level == Percent(64)
                && b.charge == Charge::Discharging
                && b.time_to_empty == Seconds(7200)
        })
        .await;
        until(
            &mut profiles_rx,
            "the power profiles",
            |p: &PowerProfiles| {
                p.presence == Presence::Present
                    && p.active == Profile::Balanced
                    && p.offered == [Profile::PowerSaver, Profile::Balanced]
                    && p.degraded == ProfileDegraded::No
            },
        )
        .await;
        until(&mut kbd_rx, "the keyboard backlight", |k: &KbdBacklight| {
            k.presence == Presence::Present && k.level.value == 1 && k.level.max == 3
        })
        .await;

        // A command reaches the daemon.
        let _ = kbd.control.send(KbdCommand::Set(Percent(100)));
        for _ in 0..100 {
            if lock(&shared).kbd_steps == [3] {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(lock(&shared).kbd_steps, [3]);

        // UPower leaves: all three go back to absent.
        drop(upower);
        until(&mut battery_rx, "the battery to go", |b: &Battery| {
            b.presence == Presence::Absent
        })
        .await;
        until(
            &mut profiles_rx,
            "the profiles to go",
            |p: &PowerProfiles| p.presence == Presence::Absent,
        )
        .await;
        until(&mut kbd_rx, "the backlight to go", |k: &KbdBacklight| {
            k.presence == Presence::Absent
        })
        .await;
    });
}
