//! Opening a file, its folder or an address with the desktop's default handler, for the
//! launcher's rows:
//!
//! - `Viewer(open)` and `Play(path)`: a call to the viewer (`viewer.rs`); when it does not take
//!   it, `xdg-open <path>` as for a `File`.
//! - `File(path)` and `Url(url)`: `xdg-open <path|url>` in a transient `systemd-run --user
//!   --scope` like an app launch, with `XDG_ACTIVATION_TOKEN` set to the request's token.
//! - `Reveal(path)`: `org.freedesktop.FileManager1.ShowItems([file uri], startup id = token)`
//!   when the name has an owner or is activatable (the call then starts the file manager), so
//!   the file manager opens the folder with the file selected; else `xdg-open` on
//!   [`reveal_folder`].
//!
//! Nothing here is logged beyond the failure's kind: a path can be private.
//!
//! The scope is used when the user manager is reachable (`$XDG_RUNTIME_DIR/systemd/private`
//! exists; `systemd-run --user` fails without it, and exits 1 like `xdg-open` does, so the
//! failure could not be told apart afterwards); otherwise, or when `systemd-run` is missing,
//! `xdg-open` runs in its own process group, as a launch does.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use sill_launcher::{AppId, OpenTarget};
use sill_model::launch::{OpenRequest, ScopeUnit, scope_unit};

use super::Reach;
use super::uri::file_uri;
use super::viewer::{Handed, hand_over, viewer_request};
use crate::env::Bus;
use crate::env::{NameState, name_state};

const FILE_MANAGER: &str = "org.freedesktop.FileManager1";

const FILE_MANAGER_PATH: &str = "/org/freedesktop/FileManager1";

/// Tells one open's scope from the next within this process.
static OPENS: AtomicU32 = AtomicU32::new(0);

/// The argument vector that opens `target` with the default handler (`xdg-open`, then the
/// path, the folder for a reveal, or the address), before the scope wrapper.
pub fn open_argv(target: &OpenTarget) -> Vec<OsString> {
    let what = match target {
        OpenTarget::File(path) | OpenTarget::Play(path) => OsString::from(path),
        OpenTarget::Viewer(open) => OsString::from(&open.file),
        OpenTarget::Reveal(path) => OsString::from(reveal_folder(path)),
        OpenTarget::Url(url) => OsString::from(url),
    };
    vec![OsString::from("xdg-open"), what]
}

/// The folder a reveal opens: the path's parent, or the path itself when it is the root or has
/// no parent.
pub fn reveal_folder(path: &Path) -> PathBuf {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(path)
        .to_path_buf()
}

/// Carry out `request`: a file for the viewer handed to it when it takes the call, a reveal
/// through the file manager when one is on the bus, else (and for a file or an address)
/// `xdg-open`.
pub(crate) async fn open(setup: &Reach, request: OpenRequest) {
    let token = request.token.as_deref();
    if let Some(call) = viewer_request(&request.target)
        && hand_over(&setup.session, &call).await == Handed::Yes
    {
        return;
    }
    if let OpenTarget::Reveal(path) = &request.target
        && show_items(&setup.session, path, token).await == Shown::Yes
    {
        return;
    }
    let wrapper = match user_manager(setup.runtime.as_deref()) {
        Manager::Reachable => Wrapper::Scope(next_unit()),
        Manager::Missing => Wrapper::Bare,
    };
    if let Err(error) = spawn_open(open_argv(&request.target), &wrapper, token) {
        eprintln!("sill: launch: opening failed ({:?})", error.kind());
    }
}

/// Whether the file manager showed the item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shown {
    Yes,
    No,
}

/// Whether the user's systemd manager can be asked for a scope (shared with `run.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Manager {
    Reachable,
    Missing,
}

/// How `xdg-open` is started.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Wrapper {
    Scope(ScopeUnit),
    /// Its own process group.
    Bare,
}

/// `FileManager1.ShowItems([uri], token)` when a file manager holds the name or can be
/// started for it.
async fn show_items(session: &Bus, path: &Path, token: Option<&str>) -> Shown {
    let shown = async {
        let Some(connection) = session.connect().await? else {
            return Ok(Shown::No);
        };
        if asks_file_manager(name_state(&connection, FILE_MANAGER).await?) == Shown::No {
            return Ok::<Shown, zbus::Error>(Shown::No);
        }
        connection
            .call_method(
                Some(FILE_MANAGER),
                FILE_MANAGER_PATH,
                Some(FILE_MANAGER),
                "ShowItems",
                &(vec![file_uri(path)], token.unwrap_or_default()),
            )
            .await?;
        Ok(Shown::Yes)
    };
    shown.await.unwrap_or(Shown::No)
}

/// Whether a file manager in `state` is asked to show the item: running or activatable, since
/// activation starts it with the file selected, as Spotlight's reveal does.
fn asks_file_manager(state: NameState) -> Shown {
    match state {
        NameState::Owned | NameState::Activatable => Shown::Yes,
        NameState::Missing => Shown::No,
    }
}

/// Whether the user's systemd manager is reachable: `systemd-run --user` connects to
/// `$XDG_RUNTIME_DIR/systemd/private`, and without it exits 1 as the command it wraps might,
/// so the failure could not be told apart afterwards.
pub(crate) fn user_manager(runtime: Option<&Path>) -> Manager {
    let socket = runtime.map(|dir| dir.join("systemd/private"));
    match socket {
        Some(socket) if socket.exists() => Manager::Reachable,
        _ => Manager::Missing,
    }
}

fn next_unit() -> ScopeUnit {
    let nonce = OPENS.fetch_add(1, Ordering::Relaxed) + 1;
    scope_unit(&AppId(format!("open.{}", std::process::id())), nonce)
}

/// `argv` as `wrapper` runs it.
fn wrapped(argv: Vec<OsString>, wrapper: &Wrapper) -> Vec<OsString> {
    match wrapper {
        Wrapper::Bare => argv,
        Wrapper::Scope(unit) => ["systemd-run", "--user", "--scope", "--collect"]
            .into_iter()
            .map(OsString::from)
            .chain([
                OsString::from(format!("--unit={}", unit.0)),
                OsString::from("--"),
            ])
            .chain(argv)
            .collect(),
    }
}

/// Start `argv` under `wrapper` with the token; when `systemd-run` itself is missing, start it
/// bare. The child is reaped on the runtime, never waited on here.
fn spawn_open(argv: Vec<OsString>, wrapper: &Wrapper, token: Option<&str>) -> std::io::Result<()> {
    let child = match spawn(&wrapped(argv.clone(), wrapper), wrapper, token) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && *wrapper != Wrapper::Bare => {
            spawn(&argv, &Wrapper::Bare, token)
        }
        result => result,
    }?;
    tokio::spawn(reap(child));
    Ok(())
}

fn spawn(
    argv: &[OsString],
    wrapper: &Wrapper,
    token: Option<&str>,
) -> std::io::Result<tokio::process::Child> {
    let (program, args) = argv.split_first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty argument vector")
    })?;
    let mut command = tokio::process::Command::new(program);
    command.args(args).stdin(std::process::Stdio::null());
    if let Some(token) = token {
        command.env("XDG_ACTIVATION_TOKEN", token);
        command.env("DESKTOP_STARTUP_ID", token);
    }
    if *wrapper == Wrapper::Bare {
        command.process_group(0);
    }
    command.kill_on_drop(false).spawn()
}

async fn reap(mut child: tokio::process::Child) {
    let _ = child.wait().await;
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use sill_launcher::OpenTarget;

    use super::{Shown, Wrapper, asks_file_manager, open_argv, reveal_folder, wrapped};
    use crate::env::NameState;
    use sill_model::launch::ScopeUnit;

    #[test]
    fn every_target_opens_with_xdg_open() {
        let argv = |parts: &[&str]| parts.iter().map(OsString::from).collect::<Vec<_>>();
        let cases: Vec<(OpenTarget, Vec<OsString>)> = vec![
            (
                OpenTarget::File(PathBuf::from("/home/p/a b.pdf")),
                argv(&["xdg-open", "/home/p/a b.pdf"]),
            ),
            (
                OpenTarget::Reveal(PathBuf::from("/home/p/docs/a.pdf")),
                argv(&["xdg-open", "/home/p/docs"]),
            ),
            (
                OpenTarget::Url("https://duckduckgo.com/?q=a%20b".to_owned()),
                argv(&["xdg-open", "https://duckduckgo.com/?q=a%20b"]),
            ),
        ];
        for (target, want) in cases {
            assert_eq!(open_argv(&target), want, "{target:?}");
        }
    }

    #[test]
    fn a_reveal_opens_the_parent_folder() {
        let cases: &[(&str, &str)] = &[
            ("/home/p/a.pdf", "/home/p"),
            ("/home/p/folder", "/home/p"),
            ("/", "/"),
            ("a.pdf", "a.pdf"),
        ];
        for &(path, want) in cases {
            assert_eq!(
                reveal_folder(Path::new(path)),
                PathBuf::from(want),
                "{path}"
            );
        }
    }

    #[test]
    fn a_running_or_activatable_file_manager_is_asked() {
        let cases = [
            (NameState::Owned, Shown::Yes),
            (NameState::Activatable, Shown::Yes),
            (NameState::Missing, Shown::No),
        ];
        for (state, want) in cases {
            assert_eq!(asks_file_manager(state), want, "{state:?}");
        }
    }

    #[test]
    fn a_scope_wraps_the_argv_and_bare_leaves_it() {
        let argv = |parts: &[&str]| parts.iter().map(OsString::from).collect::<Vec<_>>();
        let open = argv(&["xdg-open", "/home/p/a.pdf"]);
        let unit = ScopeUnit("app-sill-open.7-1.scope".to_owned());
        let cases: Vec<(Wrapper, Vec<OsString>)> = vec![
            (Wrapper::Bare, open.clone()),
            (
                Wrapper::Scope(unit),
                argv(&[
                    "systemd-run",
                    "--user",
                    "--scope",
                    "--collect",
                    "--unit=app-sill-open.7-1.scope",
                    "--",
                    "xdg-open",
                    "/home/p/a.pdf",
                ]),
            ),
        ];
        for (wrapper, want) in cases {
            assert_eq!(wrapped(open.clone(), &wrapper), want, "{wrapper:?}");
        }
    }
}
