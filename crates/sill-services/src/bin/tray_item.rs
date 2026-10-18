//! A fake status notifier item with a dbusmenu, for the tray tests and dev scripts: it registers
//! with the `StatusNotifierWatcher` on the private bus at the address it is given and logs every
//! call it receives, one line each, to the file it is given.
//!
//! The address is an argument, never `DBUS_SESSION_BUS_ADDRESS`, so this cannot reach the real
//! session bus by accident.
//!
//! The menu: `Open` (1), `Devices` (2) with the submenu `Ping` (3) and `Share` (4), a rule (5),
//! `Refresh` (6) and `Quit` (7). Clicking `Refresh` appends a row `Added` (8) and emits
//! `LayoutUpdated`, as an app does when its menu changes under an open popup.
//!
//! Log lines: `Event <id> <event>`, `AboutToShow <id>`, `Activate <x> <y>`,
//! `SecondaryActivate <x> <y>`, `ContextMenu <x> <y>`, `Scroll <delta> <orientation>`.
//!
//! Usage: `cargo run -p sill-services --bin tray_item -- <bus address> <log file>`

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use zbus::fdo::DBusProxy;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{Array, ObjectPath, Signature, StructureBuilder, Type, Value};

const ITEM_PATH: &str = "/StatusNotifierItem";
const MENU_PATH: &str = "/MenuBar";
const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const REFRESH: i32 = 6;
const ADDED: i32 = 8;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(address), Some(log)) = (args.next(), args.next()) else {
        eprintln!("usage: tray_item <private bus address> <log file>");
        std::process::exit(2);
    };
    let log = Log(PathBuf::from(log));
    let menu = Menu {
        log: log.clone(),
        grown: Arc::new(Mutex::new(false)),
    };
    let item = Item { log };
    let connection = zbus::connection::Builder::address(address.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        .serve_at(ITEM_PATH, item)
        .unwrap_or_else(|e| panic!("{e}"))
        .serve_at(MENU_PATH, menu)
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting to {address}: {e}"));
    let bus = DBusProxy::new(&connection)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let watcher = zbus::names::BusName::try_from(WATCHER).unwrap_or_else(|e| panic!("{e}"));
    for _ in 0..200 {
        if bus.name_has_owner(watcher.clone()).await.unwrap_or(false) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    connection
        .call_method(
            Some(WATCHER),
            "/StatusNotifierWatcher",
            Some(WATCHER),
            "RegisterStatusNotifierItem",
            &(ITEM_PATH,),
        )
        .await
        .unwrap_or_else(|e| panic!("registering the item: {e}"));
    println!("tray_item: registered as pid {}", std::process::id());
    std::future::pending::<()>().await;
}

/// The call log, appended one line at a time.
#[derive(Clone)]
struct Log(PathBuf);

impl Log {
    fn line(&self, text: &str) {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.0)
            .unwrap_or_else(|e| panic!("{}: {e}", self.0.display()));
        writeln!(file, "{text}").unwrap_or_else(|e| panic!("{e}"));
    }
}

/// `org.kde.StatusNotifierItem`.
struct Item {
    log: Log,
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl Item {
    fn activate(&self, x: i32, y: i32) {
        self.log.line(&format!("Activate {x} {y}"));
    }

    fn secondary_activate(&self, x: i32, y: i32) {
        self.log.line(&format!("SecondaryActivate {x} {y}"));
    }

    fn context_menu(&self, x: i32, y: i32) {
        self.log.line(&format!("ContextMenu {x} {y}"));
    }

    fn scroll(&self, delta: i32, orientation: &str) {
        self.log.line(&format!("Scroll {delta} {orientation}"));
    }

    #[zbus(property)]
    fn category(&self) -> String {
        "ApplicationStatus".to_owned()
    }

    #[zbus(property)]
    fn id(&self) -> String {
        "sill-fake-item".to_owned()
    }

    #[zbus(property)]
    fn title(&self) -> String {
        "Fake item".to_owned()
    }

    #[zbus(property)]
    fn status(&self) -> String {
        "Active".to_owned()
    }

    #[zbus(property)]
    fn window_id(&self) -> i32 {
        0
    }

    #[zbus(property)]
    fn icon_name(&self) -> String {
        "applications-other".to_owned()
    }

    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn menu(&self) -> ObjectPath<'static> {
        ObjectPath::from_static_str_unchecked(MENU_PATH)
    }
}

/// `com.canonical.dbusmenu`; `grown` is whether `Refresh` has added its row.
struct Menu {
    log: Log,
    grown: Arc<Mutex<bool>>,
}

#[interface(name = "com.canonical.dbusmenu")]
impl Menu {
    fn get_layout(
        &self,
        parent_id: i32,
        _recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> zbus::fdo::Result<(u32, Node)> {
        if parent_id != 0 {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "only the root is served, not {parent_id}"
            )));
        }
        let grown = *self
            .grown
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok((u32::from(grown) + 1, layout(grown)))
    }

    async fn event(
        &self,
        id: i32,
        event_id: &str,
        _data: Value<'_>,
        _timestamp: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.log.line(&format!("Event {id} {event_id}"));
        if id == REFRESH && event_id == "clicked" {
            *self
                .grown
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
            let _ = Self::layout_updated(&emitter, 2, 0).await;
        }
    }

    fn about_to_show(&self, id: i32) -> bool {
        self.log.line(&format!("AboutToShow {id}"));
        false
    }

    #[zbus(property)]
    fn version(&self) -> u32 {
        3
    }

    #[zbus(property)]
    fn text_direction(&self) -> String {
        "ltr".to_owned()
    }

    #[zbus(property)]
    fn status(&self) -> String {
        "normal".to_owned()
    }

    #[zbus(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        Vec::new()
    }

    #[zbus(signal)]
    async fn layout_updated(
        emitter: &SignalEmitter<'_>,
        revision: u32,
        parent: i32,
    ) -> zbus::Result<()>;
}

/// The menu's root, as `GetLayout` returns it: the node `(ia{sv}av)`.
#[derive(Serialize, Type)]
struct Node {
    id: i32,
    properties: HashMap<String, Value<'static>>,
    children: Vec<Value<'static>>,
}

/// A node's properties by name.
fn properties(props: &[(&str, &str)]) -> HashMap<String, Value<'static>> {
    props
        .iter()
        .map(|(name, value)| ((*name).to_owned(), Value::from((*value).to_owned())))
        .collect()
}

/// One dbusmenu node `(ia{sv}av)` as the variant its parent's `av` holds.
fn child(id: i32, props: &[(&str, &str)], children: Vec<Value<'static>>) -> Value<'static> {
    let mut kids = Array::new(&Signature::Variant);
    for kid in children {
        kids.append(kid).unwrap_or_else(|e| panic!("{e}"));
    }
    let structure = StructureBuilder::new()
        .add_field(id)
        .add_field(properties(props))
        .append_field(Value::Array(kids))
        .build()
        .unwrap_or_else(|e| panic!("{e}"));
    Value::Value(Box::new(Value::Structure(structure)))
}

fn row(id: i32, label: &str, children: Vec<Value<'static>>) -> Value<'static> {
    let mut props = vec![("label", label)];
    if !children.is_empty() {
        props.push(("children-display", "submenu"));
    }
    child(id, &props, children)
}

/// The whole menu, with `Added` after `Quit` once `Refresh` has been clicked.
fn layout(grown: bool) -> Node {
    let mut rows = vec![
        row(1, "Open", Vec::new()),
        row(
            2,
            "Devices",
            vec![row(3, "Ping", Vec::new()), row(4, "Share", Vec::new())],
        ),
        child(5, &[("type", "separator")], Vec::new()),
        row(REFRESH, "Refresh", Vec::new()),
        row(7, "Quit", Vec::new()),
    ];
    if grown {
        rows.push(row(ADDED, "Added", Vec::new()));
    }
    Node {
        id: 0,
        properties: properties(&[("children-display", "submenu")]),
        children: rows,
    }
}
