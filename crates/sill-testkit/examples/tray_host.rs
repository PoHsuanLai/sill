//! sill's tray service (the SNI host) on the session bus named by `DBUS_SESSION_BUS_ADDRESS`,
//! which `system-tray` reads itself and sill's tray backend therefore cannot be pointed at
//! another way. The tray test starts this as a child with only the child's environment naming a
//! private bus; it refuses to run without that variable, so it never falls back to the real
//! session bus.
//!
//! Stdin: one `TrayCommand` as JSON per line. Stdout: the `Tray` as JSON, one line each time it
//! changes.
//!
//! Usage: `DBUS_SESSION_BUS_ADDRESS=<private bus> cargo run -p sill-testkit --example tray_host -- <scratch dir>`

use std::io::{BufRead, Write};
use std::path::PathBuf;

use sill_model::tray::TrayCommand;
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::service::spawn;
use sill_services::tray::TrayService;

fn say(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn main() {
    let (Some(scratch), Some(bus)) = (
        std::env::args().nth(1),
        std::env::var_os("DBUS_SESSION_BUS_ADDRESS"),
    ) else {
        eprintln!("usage: DBUS_SESSION_BUS_ADDRESS=<private bus> tray_host <scratch dir>");
        std::process::exit(2);
    };
    if !bus.to_string_lossy().starts_with("unix:") {
        eprintln!("tray_host: not a bus address: {bus:?}");
        std::process::exit(2);
    }
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Usual),
        ..Env::isolated(&PathBuf::from(scratch))
    };
    let service = spawn::<TrayService>(runtime.handle(), &env, ());
    let mut state = service.state;
    let control = service.control;
    runtime.spawn(async move {
        loop {
            let line = serde_json::to_string(&*state.borrow_and_update())
                .unwrap_or_else(|e| panic!("{e}"));
            say(&line);
            if state.changed().await.is_err() {
                break;
            }
        }
    });
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        match serde_json::from_str::<TrayCommand>(&line) {
            Ok(command) => {
                let _ = control.send(command);
            }
            Err(error) => eprintln!("tray_host: not a command: {error}: {line}"),
        }
    }
}
