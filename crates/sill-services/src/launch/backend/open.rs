//! Opening a file, its folder or an address with the desktop's default handler, for the
//! launcher's rows:
//!
//! - `Viewer(open)` and `Play(path)`: a call to the viewer (`viewer.rs`); when it does not take
//!   it, `xdg-open <path>` as for a `File`.
//! - `File(path)` and `Url(url)`: `xdg-open <path|url>` in a transient `systemd-run --user
//!   --scope` like an app launch, with `XDG_ACTIVATION_TOKEN` set to the request's token.
//! - `Reveal(path)`: `org.freedesktop.FileManager1.ShowItems([file uri], startup id = token)`
//!   to the desktop's own Files (`org.quire.Files`) when it is installed, else to whichever
//!   file manager holds `org.freedesktop.FileManager1`; a name with an owner or an activatable
//!   service is asked (the call then starts it), so the file manager opens the folder with the
//!   file selected; else `xdg-open` on [`reveal_folder`].
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
use zbus::Connection;

use super::Reach;
use super::uri::file_uri;
use super::viewer::{Handed, hand_over, viewer_request};
use crate::deadline::{BUS_CALL, within};
use crate::env::{Bus, Children, ConnectError, NameState, ProcessGroup, Start, name_state};

/// The desktop's own file manager, asked before any other: it serves `FileManager1` on its
/// own name too, so another file manager holding the shared name does not take its place.
const DESKTOP_FILES: &str = "org.quire.Files";

/// The name any file manager offers.
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
    if let Err(error) = spawn_open(
        setup.children.as_ref(),
        open_argv(&request.target),
        &wrapper,
        token,
    ) {
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

impl Wrapper {
    /// The group the child is started in: systemd isolates a scope, a bare child leads its own.
    fn group(&self) -> ProcessGroup {
        match self {
            Wrapper::Scope(_) => ProcessGroup::Shared,
            Wrapper::Bare => ProcessGroup::Own,
        }
    }
}

/// `FileManager1.ShowItems([uri], token)` when a file manager holds the name or can be
/// started for it.
async fn show_items(session: &Bus, path: &Path, token: Option<&str>) -> Shown {
    let shown = async {
        let connection = match session.try_connect().await {
            Ok(connection) => connection,
            Err(ConnectError::NoBus(_)) => return Ok(Shown::No),
            Err(ConnectError::Refused(why)) => return Err(zbus::Error::Failure(why)),
        };
        let Some(name) = file_manager(&connection).await? else {
            return Ok::<Shown, zbus::Error>(Shown::No);
        };
        show_items_at(
            &connection,
            name,
            vec![file_uri(path)],
            token.unwrap_or_default(),
        )
        .await?;
        Ok(Shown::Yes)
    };
    match within(BUS_CALL, "the file manager", shown).await {
        Ok(Ok(shown)) => shown,
        Ok(Err(_)) | Err(_) => Shown::No,
    }
}

/// The file manager to ask: the desktop's own Files, else any, the first that is running or
/// can be started; none when no file manager is on the bus.
pub(super) async fn file_manager(connection: &Connection) -> zbus::Result<Option<&'static str>> {
    for name in [DESKTOP_FILES, FILE_MANAGER] {
        if asks_file_manager(name_state(connection, name).await?) == Shown::Yes {
            return Ok(Some(name));
        }
    }
    Ok(None)
}

/// `FileManager1.ShowItems(uris, startup)` on the file manager that owns `name`.
pub(super) async fn show_items_at(
    connection: &Connection,
    name: &str,
    uris: Vec<String>,
    startup: &str,
) -> zbus::Result<()> {
    connection
        .call_method(
            Some(name),
            FILE_MANAGER_PATH,
            Some(FILE_MANAGER),
            "ShowItems",
            &(uris, startup),
        )
        .await?;
    Ok(())
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
        .unwrap_or_else(|_| unreachable!("`open.<pid>` is a non-empty app id"))
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
/// bare. The child is reaped by `children`, never waited on here.
fn spawn_open(
    children: &dyn Children,
    argv: Vec<OsString>,
    wrapper: &Wrapper,
    token: Option<&str>,
) -> std::io::Result<()> {
    let start = |argv: Vec<OsString>, group: ProcessGroup| Start {
        argv,
        token: token.map(str::to_owned),
        group,
    };
    match children.detach(&start(wrapped(argv.clone(), wrapper), wrapper.group())) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && *wrapper != Wrapper::Bare => {
            children.detach(&start(argv, ProcessGroup::Own))
        }
        result => result,
    }
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
