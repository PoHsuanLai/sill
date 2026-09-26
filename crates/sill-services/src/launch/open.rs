//! Opening a file, its folder or an address with the desktop's default handler, for the
//! launcher's M9 rows (FINDINGS "M9 freeze" F643). Lane a fills it:
//!
//! - `File(path)` and `Url(url)`: `xdg-open <path|url>` in a transient `systemd-run --user
//!   --scope` like an app launch, with `XDG_ACTIVATION_TOKEN` set to the request's token.
//! - `Reveal(path)`: `org.freedesktop.FileManager1.ShowItems([file uri], startup id = token)`
//!   when the name has an owner, so the file manager opens the folder with the file selected;
//!   else `xdg-open` on [`reveal_folder`].
//!
//! Nothing here is logged beyond the failure's kind: a path can be private.
//!
//! The scope is used when the user manager is reachable (`$XDG_RUNTIME_DIR/systemd/private`
//! exists; `systemd-run --user` fails without it, and exits 1 like `xdg-open` does, so the
//! failure could not be told apart afterwards); otherwise, or when `systemd-run` is missing,
//! `xdg-open` runs in its own process group, as a launch does (FINDINGS "M9 lane a").

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use serde::{Deserialize, Serialize};
use sill_launcher::{AppId, OpenTarget};
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

use super::model::ScopeUnit;
use super::plan::scope_unit;
use crate::providers::files::tracker::file_uri;

const FILE_MANAGER: &str = "org.freedesktop.FileManager1";
const FILE_MANAGER_PATH: &str = "/org/freedesktop/FileManager1";

/// Tells one open's scope from the next within this process.
static OPENS: AtomicU32 = AtomicU32::new(0);

/// One open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenRequest {
    pub target: OpenTarget,
    /// The xdg-activation token the launcher's surface got before it hid.
    pub token: Option<String>,
}

/// The argument vector that opens `target` with the default handler (`xdg-open`, then the
/// path, the folder for a reveal, or the address), before the scope wrapper.
pub fn open_argv(target: &OpenTarget) -> Vec<OsString> {
    let what = match target {
        OpenTarget::File(path) => OsString::from(path),
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

/// Carry out `request`: a reveal through the file manager when one is on the bus, else (and
/// for a file or an address) `xdg-open`.
pub(super) async fn open(request: OpenRequest) {
    let token = request.token.as_deref();
    if let OpenTarget::Reveal(path) = &request.target
        && show_items(path, token).await == Shown::Yes
    {
        return;
    }
    let wrapper = match user_manager() {
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

/// Whether the user's systemd manager can be asked for a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Manager {
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

/// `FileManager1.ShowItems([uri], token)` when the name has an owner.
async fn show_items(path: &Path, token: Option<&str>) -> Shown {
    let shown = async {
        let connection = Connection::session().await?;
        let dbus = DBusProxy::new(&connection).await?;
        if !dbus
            .name_has_owner(BusName::try_from(FILE_MANAGER)?)
            .await?
        {
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

fn user_manager() -> Manager {
    let socket =
        std::env::var_os("XDG_RUNTIME_DIR").map(|dir| PathBuf::from(dir).join("systemd/private"));
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

    use super::{Wrapper, open_argv, reveal_folder, wrapped};
    use crate::launch::model::ScopeUnit;

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
