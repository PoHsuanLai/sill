//! sill's tray service against `sill-testkit`'s fake status notifier item on a private bus,
//! never the real session bus: the item's dbusmenu arrives with its submenu, a click on a row
//! of the submenu reaches the item as `Event "clicked"`, the item is asked for its own menu
//! (`ContextMenu`), told its menu is about to show, and a `LayoutUpdated` it emits is followed.
//! The SNI host reads the process's own session bus, so it runs as a child
//! (`examples/tray_host.rs`) whose environment alone names the private bus. The test needs a
//! `dbus-daemon` and skips, saying so, without one.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use sill_model::tray::{Tray, TrayCommand, TrayItemId, TrayMenuEntry, TrayMenuKind};
use sill_testkit::{PrivateBus, TempDir};

const WAIT: Duration = Duration::from_secs(20);

/// An example built, and where it is.
fn example(name: &str) -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(cargo)
        .args(["build", "-q", "-p", "sill-testkit", "--example", name])
        .status()
        .unwrap_or_else(|e| panic!("building {name}: {e}"));
    assert!(status.success(), "building {name}");
    let target = std::env::var("CARGO_TARGET_DIR").map_or_else(
        |_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"),
        PathBuf::from,
    );
    target.join("debug/examples").join(name)
}

/// The host and the item running on one private bus.
struct Fixture {
    host: Child,
    item: Child,
    commands: ChildStdin,
    states: mpsc::Receiver<Tray>,
    log: PathBuf,
}

impl Fixture {
    fn start(bus: &PrivateBus, scratch: &Path) -> Fixture {
        let mut host = Command::new(example("tray_host"))
            .arg(scratch)
            .env("DBUS_SESSION_BUS_ADDRESS", &bus.address)
            .env_remove("DBUS_SYSTEM_BUS_ADDRESS")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| panic!("starting tray_host: {e}"));
        let stdout = host.stdout.take().unwrap_or_else(|| panic!("no stdout"));
        let commands = host.stdin.take().unwrap_or_else(|| panic!("no stdin"));
        let (tx, states) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                match serde_json::from_str::<Tray>(&line) {
                    Ok(tray) => {
                        if tx.send(tray).is_err() {
                            break;
                        }
                    }
                    Err(e) => panic!("tray_host printed {line:?}: {e}"),
                }
            }
        });
        let log = scratch.join("item.log");
        let mut item = Command::new(example("tray_item"))
            .arg(&bus.address)
            .arg(&log)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| panic!("starting tray_item: {e}"));
        let stdout = item.stdout.take().unwrap_or_else(|| panic!("no stdout"));
        let mut line = String::new();
        BufReader::new(stdout)
            .read_line(&mut line)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(line.starts_with("tray_item: registered"), "{line:?}");
        Fixture {
            host,
            item,
            commands,
            states,
            log,
        }
    }

    /// The next tray state satisfying `ok`, within [`WAIT`].
    fn tray_where(&self, what: &str, ok: impl Fn(&Tray) -> bool) -> Tray {
        let end = Instant::now() + WAIT;
        let mut last = None;
        while let Some(left) = end.checked_duration_since(Instant::now()) {
            match self.states.recv_timeout(left) {
                Ok(tray) if ok(&tray) => return tray,
                Ok(tray) => last = Some(tray),
                Err(_) => break,
            }
        }
        panic!("timed out waiting for {what}; last tray {last:?}");
    }

    fn send(&mut self, command: &TrayCommand) {
        let line = serde_json::to_string(command).unwrap_or_else(|e| panic!("{e}"));
        writeln!(self.commands, "{line}").unwrap_or_else(|e| panic!("{e}"));
        self.commands.flush().unwrap_or_else(|e| panic!("{e}"));
    }

    /// The item's log once it has `line`, within [`WAIT`].
    fn logged(&self, line: &str) -> Vec<String> {
        let end = Instant::now() + WAIT;
        loop {
            let lines: Vec<String> = std::fs::read_to_string(&self.log)
                .unwrap_or_default()
                .lines()
                .map(str::to_owned)
                .collect();
            if lines.iter().any(|l| l == line) {
                return lines;
            }
            assert!(
                Instant::now() < end,
                "the item never logged {line:?}: {lines:?}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.item.kill();
        let _ = self.item.wait();
        let _ = self.host.kill();
        let _ = self.host.wait();
    }
}

/// `rows` as `label(children)` words, rules as `-`, in order, ids after `#`.
fn outline(rows: &[TrayMenuEntry]) -> String {
    rows.iter()
        .map(|row| match &row.kind {
            TrayMenuKind::Separator => format!("-#{}", row.id),
            TrayMenuKind::Item {
                label, children, ..
            } if children.is_empty() => format!("{label}#{}", row.id),
            TrayMenuKind::Item {
                label, children, ..
            } => format!("{label}#{}({})", row.id, outline(children)),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

const MENU: &str = "Open#1 Devices#2(Ping#3 Share#4) -#5 Refresh#6 Quit#7";

#[test]
fn a_submenu_row_click_reaches_the_item_and_its_context_menu_and_layout_updates_follow() {
    let Some(bus) = PrivateBus::start() else {
        return;
    };
    let scratch = TempDir::new("tray-bus");
    let mut fixture = Fixture::start(&bus, scratch.path());

    // The item and its whole menu, submenu included, arrive.
    let tray = fixture.tray_where("the item's menu", |t| {
        t.items.iter().any(|i| !i.menu.is_empty())
    });
    let item = tray.items[0].id.clone();
    assert_eq!(outline(&tray.items[0].menu), MENU);

    // The menu is about to show.
    fixture.send(&TrayCommand::MenuOpening { item: item.clone() });
    fixture.logged("AboutToShow 0");

    // A click on a row of the submenu is the item's `Event(3, "clicked")`.
    fixture.send(&TrayCommand::MenuPick {
        item: item.clone(),
        entry: 3,
    });
    fixture.logged("Event 3 clicked");

    // An item with no dbusmenu of its own to show is asked for its menu where the press was.
    fixture.send(&TrayCommand::ContextMenu {
        item: item.clone(),
        x: 1900,
        y: 12,
    });
    let lines = fixture.logged("ContextMenu 1900 12");
    assert_eq!(
        lines.iter().filter(|l| l.starts_with("Event")).count(),
        1,
        "the context menu is no menu click: {lines:?}"
    );

    // The app changes its menu under the open popup: LayoutUpdated, and the tray follows.
    fixture.send(&TrayCommand::MenuPick {
        item: TrayItemId(item.0.clone()),
        entry: 6,
    });
    let tray = fixture.tray_where("the grown menu", |t| {
        t.items
            .first()
            .is_some_and(|i| outline(&i.menu).ends_with("Quit#7 Added#8"))
    });
    assert_eq!(
        outline(&tray.items[0].menu),
        format!("{MENU} Added#8"),
        "the whole menu is replaced, not patched"
    );
}
