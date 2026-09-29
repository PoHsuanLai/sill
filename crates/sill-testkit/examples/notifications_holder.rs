//! Another notification daemon, as far as the bus can tell: holds `org.freedesktop.Notifications`
//! on the private bus at the address it is given until it is killed, so
//! `dev/accept/notifications.sh --bus` can check that sill reports it and never replaces it.
//!
//! The address is an argument, never `DBUS_SESSION_BUS_ADDRESS`, so this cannot reach the real
//! session bus by accident (plasmashell holds the name there).
//!
//! Usage: `cargo run -p sill-testkit --example notifications_holder -- <bus address>`

use std::time::Duration;

use zbus::fdo::{RequestNameFlags, RequestNameReply};

const NAME: &str = "org.freedesktop.Notifications";

fn main() {
    let Some(address) = std::env::args().nth(1) else {
        eprintln!("usage: notifications_holder <private bus address>");
        std::process::exit(2);
    };
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    runtime.block_on(async {
        let connection = zbus::connection::Builder::address(address.as_str())
            .unwrap_or_else(|e| panic!("{e}"))
            .build()
            .await
            .unwrap_or_else(|e| panic!("connecting to {address}: {e}"));
        // Never `ReplaceExisting`: this stands in for a daemon that got there first.
        let reply = connection
            .request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into())
            .await
            .unwrap_or_else(|e| panic!("requesting {NAME}: {e}"));
        assert_eq!(reply, RequestNameReply::PrimaryOwner, "{NAME} was taken");
        println!("holding {NAME} as pid {}", std::process::id());
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}
