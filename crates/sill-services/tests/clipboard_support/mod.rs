//! The clipboard live test's reach into the nested compositor: its socket (by absolute path,
//! pinged first, never `wayland-0`), `wl-copy` and `wl-paste` pointed at it, and the
//! `secret_source` example. What is copied is made up; nothing read back is ever printed.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// The nested compositor's socket: `$SILL_NESTED_SOCKET`, else `$XDG_RUNTIME_DIR/wayland-1`.
/// Panics (the test is ignored unless asked for) when it is not absolute, is the session's
/// `wayland-0`, or does not answer a connection.
pub fn nested_socket() -> PathBuf {
    let socket = std::env::var_os("SILL_NESTED_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let runtime = std::env::var_os("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR is set");
            PathBuf::from(runtime).join("wayland-1")
        });
    assert!(
        socket.is_absolute(),
        "the nested socket is named by absolute path"
    );
    assert!(
        socket.file_name().is_some_and(|name| name != "wayland-0"),
        "never the session's own compositor"
    );
    UnixStream::connect(&socket).unwrap_or_else(|e| {
        panic!(
            "the nested compositor at {} does not answer ({e}); start dev/nested.sh",
            socket.display()
        )
    });
    socket
}

/// A `wl-*` command against the nested compositor only.
fn wl(program: &str, socket: &Path) -> Command {
    let mut command = Command::new(program);
    command.env("WAYLAND_DISPLAY", socket).stderr(Stdio::null());
    command
}

/// `wl-copy` of `text` (its own background process serves it until it is replaced).
pub fn copy_text(socket: &Path, text: &str) {
    let status = wl("wl-copy", socket)
        .arg("--")
        .arg(text)
        .stdout(Stdio::null())
        .status()
        .expect("wl-copy runs (is wl-clipboard installed?)");
    assert!(status.success(), "wl-copy failed: {status}");
}

/// `wl-copy --type image/png` of `png`.
pub fn copy_png(socket: &Path, png: &[u8]) {
    let mut child = wl("wl-copy", socket)
        .args(["--type", "image/png"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .expect("wl-copy runs");
    child
        .stdin
        .take()
        .expect("wl-copy's stdin")
        .write_all(png)
        .expect("the image is handed to wl-copy");
    let status = child.wait().expect("wl-copy ends");
    assert!(status.success(), "wl-copy failed: {status}");
}

/// What `wl-paste` reads as text from the nested selection (the caller compares it; it is
/// never printed).
pub fn paste_text(socket: &Path) -> Vec<u8> {
    wl("wl-paste", socket)
        .args(["--no-newline", "--type", "text"])
        .output()
        .map(|out| out.stdout)
        .unwrap_or_default()
}

/// `wl-copy --clear`: the nested selection empty again.
pub fn clear(socket: &Path) {
    let _ = wl("wl-copy", socket)
        .arg("--clear")
        .stdout(Stdio::null())
        .status();
}

/// The `secret_source` example serving its marked text, once it says `ready`.
pub fn secret_source(socket: &Path) -> Child {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut child = Command::new(cargo)
        .args([
            "run",
            "--quiet",
            "-p",
            "sill-services",
            "--example",
            "secret_source",
            "--",
        ])
        .arg(socket)
        .arg("30")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("cargo runs the secret_source example");
    let stdout = child.stdout.take().expect("the example's stdout");
    let mut line = String::new();
    BufReader::new(stdout)
        .read_line(&mut line)
        .expect("the example says it is ready");
    assert_eq!(line.trim(), "ready", "the example set its selection");
    child
}

/// A `width` x `height` PNG, made here.
pub fn png(width: u32, height: u32) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::RgbaImage::new(width, height)
        .write_to(&mut out, image::ImageFormat::Png)
        .expect("an in-memory PNG encodes");
    out.into_inner()
}
