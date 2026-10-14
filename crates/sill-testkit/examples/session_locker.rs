//! A password-free `ext-session-lock-v1` locker for tests, never for a real session:
//!
//! ```sh
//! WAYLAND_DISPLAY=wayland-1 cargo run -p sill-testkit --example session_locker -- <hold-ms>
//! ```
//!
//! Locks the compositor, gives every output a lock surface (one plain colour, as a real locker
//! does: a compositor such as casement sends `locked` only once each output shows one), prints
//! `locked` on stdout once the compositor says so, holds the lock for `hold-ms`, then unlocks and
//! exits 0. `0` is the recovery form: a locker killed without unlocking leaves the compositor
//! locked until another client locks and unlocks (its session-lock handler accepts a new lock
//! once the old client is gone), and this is that client. Exit 3 when the compositor refused the
//! lock (`finished`: someone else holds it), 2 on a usage or connection error.
//!
//! `dev/accept/lock.sh` and `dev/accept/idle.sh` run it on the compositor under test.

use std::fs::File;
use std::io::Write;
use std::os::fd::AsFd;
use std::process::ExitCode;
use std::time::Duration;

use rustix::fs::{MemfdFlags, memfd_create};
use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::{
    wl_buffer::WlBuffer,
    wl_compositor::WlCompositor,
    wl_output::WlOutput,
    wl_registry,
    wl_shm::{self, WlShm},
    wl_shm_pool::WlShmPool,
    wl_surface::WlSurface,
};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1::ExtSessionLockManagerV1,
    ext_session_lock_surface_v1::{self, ExtSessionLockSurfaceV1},
    ext_session_lock_v1::{self, ExtSessionLockV1},
};

/// The lock surfaces' plain colour: opaque, dark grey (ARGB8888, little endian).
const GROUND: [u8; 4] = [0x20, 0x20, 0x20, 0xFF];
/// Bytes per ARGB8888 pixel.
const PIXEL_BYTES: usize = 4;

/// What the compositor has said about our lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Waiting,
    Locked,
    Refused,
}

#[derive(Debug)]
struct State {
    answer: Answer,
    shm: WlShm,
    /// The buffers on the lock surfaces, kept alive until the unlock.
    buffers: Vec<WlBuffer>,
}

fn main() -> ExitCode {
    let Some(hold) = std::env::args()
        .nth(1)
        .and_then(|ms| ms.parse::<u64>().ok())
        .map(Duration::from_millis)
    else {
        eprintln!("usage: session_locker <hold-ms>");
        return ExitCode::from(2);
    };
    match run(hold) {
        Ok(Answer::Locked) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(3),
        Err(why) => {
            eprintln!("session_locker: {why}");
            ExitCode::from(2)
        }
    }
}

fn run(hold: Duration) -> Result<Answer, String> {
    let connection = Connection::connect_to_env().map_err(|e| e.to_string())?;
    let (globals, mut queue) =
        registry_queue_init::<State>(&connection).map_err(|e| e.to_string())?;
    let handle = queue.handle();
    let manager: ExtSessionLockManagerV1 = globals
        .bind(&handle, 1..=1, ())
        .map_err(|e| format!("no ext_session_lock_manager_v1: {e}"))?;
    let compositor: WlCompositor = globals
        .bind(&handle, 1..=6, ())
        .map_err(|e| format!("no wl_compositor: {e}"))?;
    let shm: WlShm = globals
        .bind(&handle, 1..=1, ())
        .map_err(|e| format!("no wl_shm: {e}"))?;
    let lock = manager.lock(&handle, ());
    let surfaces = lock_surfaces(&globals, &handle, &compositor, &lock);
    let mut state = State {
        answer: Answer::Waiting,
        shm,
        buffers: Vec::new(),
    };
    while state.answer == Answer::Waiting {
        queue
            .blocking_dispatch(&mut state)
            .map_err(|e| e.to_string())?;
    }
    if state.answer == Answer::Refused {
        lock.destroy();
        let _ = queue.roundtrip(&mut state);
        return Ok(Answer::Refused);
    }
    println!("locked");
    std::thread::sleep(hold);
    surfaces.iter().for_each(ExtSessionLockSurfaceV1::destroy);
    lock.unlock_and_destroy();
    finish(&mut queue, &mut state)?;
    println!("unlocked");
    Ok(Answer::Locked)
}

/// One lock surface per output the compositor announced; each is painted when it is configured.
fn lock_surfaces(
    globals: &GlobalList,
    handle: &QueueHandle<State>,
    compositor: &WlCompositor,
    lock: &ExtSessionLockV1,
) -> Vec<ExtSessionLockSurfaceV1> {
    let outputs = globals.contents().clone_list();
    outputs
        .iter()
        .filter(|global| global.interface == WlOutput::interface().name)
        .map(|global| {
            let output: WlOutput =
                globals
                    .registry()
                    .bind(global.name, global.version.min(4), handle, ());
            let surface = compositor.create_surface(handle, ());
            lock.get_lock_surface(&surface, &output, handle, surface.clone())
        })
        .collect()
}

/// A `width` x `height` buffer of the plain colour, on `shm`.
fn painted_buffer(
    shm: &WlShm,
    handle: &QueueHandle<State>,
    width: u32,
    height: u32,
) -> Result<WlBuffer, String> {
    let stride = usize::try_from(width).map_err(|e| e.to_string())? * PIXEL_BYTES;
    let bytes = stride * usize::try_from(height).map_err(|e| e.to_string())?;
    let size = i32::try_from(bytes).map_err(|e| e.to_string())?;
    let fd = memfd_create("sill-session-locker", MemfdFlags::CLOEXEC).map_err(|e| e.to_string())?;
    let mut file = File::from(fd);
    file.write_all(&GROUND.repeat(bytes / PIXEL_BYTES))
        .map_err(|e| e.to_string())?;
    let pool = shm.create_pool(file.as_fd(), size, handle, ());
    let buffer = pool.create_buffer(
        0,
        i32::try_from(width).map_err(|e| e.to_string())?,
        i32::try_from(height).map_err(|e| e.to_string())?,
        i32::try_from(stride).map_err(|e| e.to_string())?,
        wl_shm::Format::Argb8888,
        handle,
        (),
    );
    pool.destroy();
    Ok(buffer)
}

/// One roundtrip so the compositor has processed the unlock before the connection closes.
fn finish(queue: &mut EventQueue<State>, state: &mut State) -> Result<(), String> {
    queue
        .roundtrip(state)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtSessionLockManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ExtSessionLockManagerV1,
        _: <ExtSessionLockManagerV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtSessionLockV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.answer = match event {
            ext_session_lock_v1::Event::Locked => Answer::Locked,
            ext_session_lock_v1::Event::Finished => Answer::Refused,
            _ => state.answer,
        };
    }
}

impl Dispatch<ExtSessionLockSurfaceV1, WlSurface> for State {
    fn event(
        state: &mut Self,
        lock_surface: &ExtSessionLockSurfaceV1,
        event: ext_session_lock_surface_v1::Event,
        surface: &WlSurface,
        _: &Connection,
        handle: &QueueHandle<Self>,
    ) {
        let ext_session_lock_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        else {
            return;
        };
        lock_surface.ack_configure(serial);
        match painted_buffer(&state.shm, handle, width, height) {
            Ok(buffer) => {
                surface.attach(Some(&buffer), 0, 0);
                surface.damage_buffer(0, 0, i32::MAX, i32::MAX);
                surface.commit();
                state.buffers.push(buffer);
            }
            Err(why) => eprintln!("session_locker: painting a lock surface: {why}"),
        }
    }
}

/// Objects whose events the locker has no use for.
macro_rules! ignore_events {
    ($($interface:ty),+ $(,)?) => {$(
        impl Dispatch<$interface, ()> for State {
            fn event(
                _: &mut Self,
                _: &$interface,
                _: <$interface as Proxy>::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    )+};
}

ignore_events!(
    WlCompositor,
    WlShm,
    WlShmPool,
    WlBuffer,
    WlSurface,
    WlOutput
);
