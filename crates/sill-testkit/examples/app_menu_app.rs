//! A fake app that exports its menu bar as quire apps do (`org.quire.AppMenu.<app_id>` at
//! `/org/quire/AppMenu`, `com.canonical.dbusmenu`), for the app-menu tests and dev scripts. It
//! owns the bus name it is given on the private bus at the address it is given, prints its pid,
//! and logs every call it receives, one line each, to the file it is given.
//!
//! The address is an argument, never `DBUS_SESSION_BUS_ADDRESS`, so this cannot reach the real
//! session bus by accident.
//!
//! The bar: `Notes` (11: `About Notes` 12, `Quit Notes` 13), `Edit` (20: `Undo` 21 with Cmd+Z and
//! the command `notes.archive`, `Spoof` 22 naming the command `shell.workspace.switch`, `Refresh`
//! 23), `View` (40: `Sidebar` 41 checked, `Erase All` 42 with no command). Clicking `Refresh`
//! adds `Redo` (24) to Edit and emits `LayoutUpdated`, as an app does when its menu changes under
//! an open popup.
//!
//! Log lines: `Event <id> <event>`, `AboutToShow <id>`.
//!
//! Usage: `cargo run -p sill-testkit --example app_menu_app -- <bus address> <log file> <bus name>`

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{Array, Signature, StructureBuilder, Type, Value};

const MENU_PATH: &str = "/org/quire/AppMenu";
const REFRESH: i32 = 23;
const EDIT: i32 = 20;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(address), Some(log), Some(name)) = (args.next(), args.next(), args.next()) else {
        eprintln!("usage: app_menu_app <private bus address> <log file> <bus name>");
        std::process::exit(2);
    };
    let menu = Menu {
        log: Log(PathBuf::from(log)),
        grown: Arc::new(Mutex::new(false)),
    };
    let _connection = zbus::connection::Builder::address(address.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        .serve_at(MENU_PATH, menu)
        .unwrap_or_else(|e| panic!("{e}"))
        .name(name.as_str())
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting to {address}: {e}"));
    println!("app_menu_app: serving {name} as pid {}", std::process::id());
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
        let grown = *self.grown.lock().unwrap_or_else(|e| e.into_inner());
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
            *self.grown.lock().unwrap_or_else(|e| e.into_inner()) = true;
            let _ = Self::layout_updated(&emitter, 2, EDIT).await;
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
    fn status(&self) -> String {
        "normal".to_owned()
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
fn properties(props: Vec<(&str, Value<'static>)>) -> HashMap<String, Value<'static>> {
    props
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect()
}

/// One dbusmenu node `(ia{sv}av)` as the variant its parent's `av` holds.
fn child(
    id: i32,
    props: Vec<(&str, Value<'static>)>,
    children: Vec<Value<'static>>,
) -> Value<'static> {
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

fn text(value: &str) -> Value<'static> {
    Value::from(value.to_owned())
}

/// A row with `label` and the extra properties `more`.
fn row(id: i32, label: &str, mut more: Vec<(&'static str, Value<'static>)>) -> Value<'static> {
    more.push(("label", text(label)));
    child(id, more, Vec::new())
}

/// A menu of the bar with its rows.
fn menu(id: i32, label: &str, rows: Vec<Value<'static>>) -> Value<'static> {
    child(
        id,
        vec![
            ("label", text(label)),
            ("children-display", text("submenu")),
        ],
        rows,
    )
}

fn command(id: &str) -> (&'static str, Value<'static>) {
    ("x-quire-command-id", text(id))
}

/// The whole bar, with `Redo` after `Refresh` once it has been clicked.
fn layout(grown: bool) -> Node {
    let shortcut = Value::from(vec![vec!["Super".to_owned(), "Z".to_owned()]]);
    let mut edit = vec![
        row(
            21,
            "Undo",
            vec![("shortcut", shortcut), command("notes.archive")],
        ),
        row(22, "Spoof", vec![command("shell.workspace.switch")]),
        row(REFRESH, "Refresh", vec![command("note.refresh")]),
    ];
    if grown {
        edit.push(row(24, "Redo", vec![command("note.redo")]));
    }
    let sidebar = row(
        41,
        "Sidebar",
        vec![
            ("toggle-type", text("checkmark")),
            ("toggle-state", Value::from(1_i32)),
        ],
    );
    Node {
        id: 0,
        properties: properties(vec![("children-display", text("submenu"))]),
        children: vec![
            menu(
                11,
                "Notes",
                vec![
                    row(12, "About Notes", vec![command("app.about")]),
                    row(13, "Quit Notes", vec![command("app.quit")]),
                ],
            ),
            menu(EDIT, "Edit", edit),
            menu(40, "View", vec![sidebar, row(42, "Erase All", Vec::new())]),
        ],
    }
}
