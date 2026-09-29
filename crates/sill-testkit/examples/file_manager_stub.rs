//! A file manager, as far as the bus can tell: owns `org.freedesktop.FileManager1` on the
//! private bus at the address it is given and prints every call it gets, so
//! `dev/accept/launcher.sh` can check that ⌘R on a file row reveals it through
//! `ShowItems` with the activation token, without opening anything.
//!
//! The address is an argument, never `DBUS_SESSION_BUS_ADDRESS`, so this cannot reach the real
//! session bus by accident (the person's own file manager may hold the name there).
//!
//! Usage: `file_manager_stub <private bus address>`; prints `ready`, then one line per call:
//! `<method>\t<uri> <uri>…\t<startup id>`.

use std::io::Write;
use std::time::Duration;

use zbus::fdo::{RequestNameFlags, RequestNameReply};
use zbus::interface;

const NAME: &str = "org.freedesktop.FileManager1";
const PATH: &str = "/org/freedesktop/FileManager1";

struct Stub;

#[interface(name = "org.freedesktop.FileManager1")]
impl Stub {
    fn show_items(&self, uris: Vec<String>, startup_id: String) {
        record("ShowItems", &uris, &startup_id);
    }

    fn show_folders(&self, uris: Vec<String>, startup_id: String) {
        record("ShowFolders", &uris, &startup_id);
    }

    fn show_item_properties(&self, uris: Vec<String>, startup_id: String) {
        record("ShowItemProperties", &uris, &startup_id);
    }
}

fn record(method: &str, uris: &[String], startup_id: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{method}\t{}\t{startup_id}", uris.join(" "));
    let _ = out.flush();
}

fn main() {
    let Some(address) = std::env::args().nth(1) else {
        eprintln!("usage: file_manager_stub <private bus address>");
        std::process::exit(2);
    };
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    runtime.block_on(async {
        let connection = zbus::connection::Builder::address(address.as_str())
            .unwrap_or_else(|e| panic!("{e}"))
            .serve_at(PATH, Stub)
            .unwrap_or_else(|e| panic!("{e}"))
            .build()
            .await
            .unwrap_or_else(|e| panic!("connecting to {address}: {e}"));
        let reply = connection
            .request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into())
            .await
            .unwrap_or_else(|e| panic!("requesting {NAME}: {e}"));
        assert_eq!(reply, RequestNameReply::PrimaryOwner, "{NAME} was taken");
        record("ready", &[], "");
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}
