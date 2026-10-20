//! Next Input Source against a stand-in for fcitx5's controller on a private bus playing the
//! session bus, and a stand-in for IBus on a second private bus playing IBus's own, never a real
//! fcitx5 or IBus. The tests need a `dbus-daemon` and skip without one.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use sill_model::input_source::{InputCommand, InputRefusal, InputSwitch};
use sill_services::InputSourceService;
use sill_services::env::{BusRoute, Buses, Env, IbusRoute};
use sill_services::service::spawn;
use sill_testkit::{PrivateBus, TempDir};
use tokio::sync::watch;
use zbus::interface;
use zbus::zvariant::{StructureBuilder, Value};

/// What the stand-in heard `SetCurrentIM` with.
type Heard = Arc<Mutex<Vec<String>>>;

fn lock(heard: &Heard) -> MutexGuard<'_, Vec<String>> {
    heard.lock().unwrap_or_else(|e| e.into_inner())
}

struct Controller(Heard);

#[interface(name = "org.fcitx.Fcitx.Controller1")]
impl Controller {
    fn current_input_method(&self) -> String {
        "pinyin".to_owned()
    }

    fn current_input_method_group(&self) -> String {
        "Default".to_owned()
    }

    fn input_method_group_info(&self, _group: String) -> (String, Vec<(String, String)>) {
        let entry = |name: &str| (name.to_owned(), String::new());
        (
            "us".to_owned(),
            vec![entry("keyboard-us"), entry("pinyin"), entry("mozc")],
        )
    }

    #[zbus(name = "SetCurrentIM")]
    fn set_current_im(&self, name: String) {
        lock(&self.0).push(name);
    }
}

struct Ibus(Heard);

/// IBus's global engine description is a long structure; what is read of it is the third field,
/// the engine's name.
fn engine(name: &str) -> Value<'static> {
    let structure = StructureBuilder::new()
        .add_field("IBusEngineDesc".to_owned())
        .add_field(HashMap::<String, Value<'static>>::new())
        .add_field(name.to_owned())
        .build()
        .unwrap_or_else(|e| panic!("{e}"));
    Value::Value(Box::new(Value::Structure(structure)))
}

#[interface(name = "org.freedesktop.IBus")]
impl Ibus {
    fn list_active_engines(&self) -> Vec<Value<'static>> {
        vec![engine("xkb:us::eng"), engine("pinyin"), engine("mozc-jp")]
    }

    fn get_global_engine(&self) -> Value<'static> {
        engine("mozc-jp")
    }

    fn set_global_engine(&self, name: String) {
        lock(&self.0).push(name);
    }
}

fn env_for(bus: &PrivateBus, scratch: &TempDir) -> Env {
    Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ibus: IbusRoute::Absent,
        ..Env::isolated(scratch.path())
    }
}

async fn until(rx: &mut watch::Receiver<InputSwitch>, ok: impl FnMut(&InputSwitch) -> bool) {
    let waited = tokio::time::timeout(Duration::from_secs(10), rx.wait_for(ok)).await;
    assert!(
        matches!(waited, Ok(Ok(_))),
        "timed out waiting for the switch"
    );
}

#[test]
fn next_input_source_tells_fcitx5_the_entry_after_the_current_one_or_refuses() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("input-source");
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    rt.block_on(async {
        let env = env_for(&bus, &scratch);
        let service = spawn::<InputSourceService>(&tokio::runtime::Handle::current(), &env, ());
        let mut state = service.state.clone();

        // No framework on the bus: a refusal, not a panic.
        let _ = service.control.send(InputCommand::Next);
        until(&mut state, |s| *s != InputSwitch::Idle).await;
        assert_eq!(
            *state.borrow(),
            InputSwitch::Refused(InputRefusal::NoFramework)
        );

        let heard: Heard = Arc::default();
        let _fcitx = zbus::connection::Builder::address(bus.address.as_str())
            .and_then(|b| b.name("org.fcitx.Fcitx5"))
            .and_then(|b| b.serve_at("/controller", Controller(heard.clone())))
            .unwrap_or_else(|e| panic!("{e}"))
            .build()
            .await
            .unwrap_or_else(|e| panic!("the stand-in fcitx5: {e}"));
        let _ = service.control.send(InputCommand::Next);
        until(&mut state, |s| matches!(s, InputSwitch::Switched(_))).await;
        assert_eq!(*lock(&heard), ["mozc"]);
        assert_eq!(*state.borrow(), InputSwitch::Switched("mozc".to_owned()));
    });
}

#[test]
fn next_input_source_tells_ibus_the_engine_after_the_current_one() {
    let (Some(session), Some(ibus_bus)) = (PrivateBus::start(), PrivateBus::start()) else {
        return;
    };
    let scratch = TempDir::new("input-source-ibus");
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    rt.block_on(async {
        let env = Env {
            ibus: IbusRoute::Address(ibus_bus.address.clone()),
            ..env_for(&session, &scratch)
        };
        let heard: Heard = Arc::default();
        let _ibus = zbus::connection::Builder::address(ibus_bus.address.as_str())
            .and_then(|b| b.name("org.freedesktop.IBus"))
            .and_then(|b| b.serve_at("/org/freedesktop/IBus", Ibus(heard.clone())))
            .unwrap_or_else(|e| panic!("{e}"))
            .build()
            .await
            .unwrap_or_else(|e| panic!("the stand-in IBus: {e}"));
        let service = spawn::<InputSourceService>(&tokio::runtime::Handle::current(), &env, ());
        let mut state = service.state.clone();

        let _ = service.control.send(InputCommand::Next);
        until(&mut state, |s| matches!(s, InputSwitch::Switched(_))).await;
        assert_eq!(*lock(&heard), ["xkb:us::eng"]);
    });
}

#[test]
fn an_ibus_address_that_connects_to_nothing_is_no_framework() {
    let Some(session) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("input-source-no-ibus");
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    rt.block_on(async {
        let gone = scratch.path().join("no-ibus-socket");
        let env = Env {
            ibus: IbusRoute::Address(format!("unix:path={}", gone.display())),
            ..env_for(&session, &scratch)
        };
        let service = spawn::<InputSourceService>(&tokio::runtime::Handle::current(), &env, ());
        let mut state = service.state.clone();
        let _ = service.control.send(InputCommand::Next);
        until(&mut state, |s| *s != InputSwitch::Idle).await;
        assert_eq!(
            *state.borrow(),
            InputSwitch::Refused(InputRefusal::NoFramework)
        );
    });
}
