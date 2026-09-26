//! A stand-in password manager for the clipboard history's tests (M9 lane b): it makes a
//! made-up text the selection of a nested compositor, marked the way KeePassXC and KWallet
//! mark a copied password (`x-kde-passwordManagerHint`, value `secret`), through
//! `ext_data_control_v1` (no window or focus needed). The history must skip it unread.
//!
//! Usage: `secret_source <absolute socket path> [seconds]`. The socket must be named by an
//! absolute path and must not be `wayland-0` (the person's own session is never touched).
//! Prints `ready` once the selection is set; exits when another client takes the selection,
//! or after `seconds` (default 10).

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use cosmic_client_toolkit::wayland_client::globals::{GlobalListContents, registry_queue_init};
use cosmic_client_toolkit::wayland_client::protocol::{wl_registry::WlRegistry, wl_seat::WlSeat};
use cosmic_client_toolkit::wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use cosmic_client_toolkit::wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::{self, ExtDataControlDeviceV1},
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::ExtDataControlOfferV1,
    ext_data_control_source_v1::{self, ExtDataControlSourceV1},
};

/// What a paste of the text receives: made up, and recognisable in no one's clipboard.
const MADE_UP: &str = "m9-secret-source-made-up-7c1e";

/// The types offered: text, and the password manager's mark.
const TYPES: &[&str] = &[
    "text/plain;charset=utf-8",
    "text/plain",
    "x-kde-passwordManagerHint",
];

#[derive(Debug, Default)]
struct State {
    ended: Ending,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum Ending {
    #[default]
    Serving,
    Replaced,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let socket = PathBuf::from(args.next().unwrap_or_default());
    let seconds = args.next().and_then(|s| s.parse().ok()).unwrap_or(10);
    if let Err(refusal) = check(&socket) {
        eprintln!("secret_source: {refusal}");
        std::process::exit(2);
    }
    if let Err(error) = serve(&socket, Duration::from_secs(seconds)) {
        eprintln!("secret_source: {error}");
        std::process::exit(1);
    }
}

fn check(socket: &Path) -> Result<(), String> {
    if !socket.is_absolute() {
        return Err("give the nested compositor's socket by absolute path".to_owned());
    }
    if socket.file_name().is_some_and(|name| name == "wayland-0") {
        return Err("refusing wayland-0, the session's own compositor".to_owned());
    }
    Ok(())
}

fn serve(socket: &Path, within: Duration) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::from_socket(UnixStream::connect(socket)?)?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn)?;
    let qh = queue.handle();
    let seat = globals.bind::<WlSeat, _, _>(&qh, 1..=1, ())?;
    let manager = globals.bind::<ExtDataControlManagerV1, _, _>(&qh, 1..=1, ())?;
    let device = manager.get_data_device(&seat, &qh, ());
    let source = manager.create_data_source(&qh, ());
    TYPES
        .iter()
        .for_each(|mime| source.offer((*mime).to_owned()));
    device.set_selection(Some(&source));
    let mut state = State::default();
    queue.roundtrip(&mut state)?;
    println!("ready");
    std::io::stdout().flush()?;
    // The deadline: a watchdog ends the process however the loop is doing.
    std::thread::spawn(move || {
        std::thread::sleep(within);
        std::process::exit(0);
    });
    while state.ended == Ending::Serving {
        queue.blocking_dispatch(&mut state)?;
    }
    source.destroy();
    queue.roundtrip(&mut state)?;
    Ok(())
}

impl Dispatch<ExtDataControlSourceV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtDataControlSourceV1,
        event: ext_data_control_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_source_v1::Event::Send { mime_type, fd } => {
                let body = if mime_type == "x-kde-passwordManagerHint" {
                    "secret"
                } else {
                    MADE_UP
                };
                let _ = std::fs::File::from(fd).write_all(body.as_bytes());
            }
            ext_data_control_source_v1::Event::Cancelled => state.ended = Ending::Replaced,
            _ => {}
        }
    }
}

impl Dispatch<ExtDataControlDeviceV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ExtDataControlDeviceV1,
        event: ext_data_control_device_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Offers this client is shown are not read; it only serves its own.
        match event {
            ext_data_control_device_v1::Event::Selection { id: Some(offer) }
            | ext_data_control_device_v1::Event::PrimarySelection { id: Some(offer) } => {
                offer.destroy()
            }
            _ => {}
        }
    }

    cosmic_client_toolkit::wayland_client::event_created_child!(State, ExtDataControlDeviceV1, [
        ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ExtDataControlOfferV1, ()),
    ]);
}

/// The events every other object here has, all ignored.
macro_rules! quiet {
    ($ty:ty, $data:ty) => {
        impl Dispatch<$ty, $data> for State {
            fn event(
                _: &mut Self,
                _: &$ty,
                _: <$ty as Proxy>::Event,
                _: &$data,
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    };
}

quiet!(WlRegistry, GlobalListContents);
quiet!(WlSeat, ());
quiet!(ExtDataControlManagerV1, ());
quiet!(ExtDataControlOfferV1, ());
