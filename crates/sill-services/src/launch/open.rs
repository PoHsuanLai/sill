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

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sill_launcher::OpenTarget;

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
    let _ = target;
    todo!("M9 fill: a-files")
}

/// The folder a reveal opens: the path's parent, or the path itself when it is the root or has
/// no parent.
pub fn reveal_folder(path: &Path) -> PathBuf {
    let _ = path;
    todo!("M9 fill: a-files")
}

/// Carry out `request`. The freeze logs that it cannot yet; lane a runs it.
pub(super) async fn open(request: OpenRequest) {
    let kind = match request.target {
        OpenTarget::File(_) => "a file",
        OpenTarget::Reveal(_) => "a folder",
        OpenTarget::Url(_) => "an address",
    };
    eprintln!("sill: launch: opening {kind} is not filled yet (M9 lane a)");
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use sill_launcher::OpenTarget;

    use super::{open_argv, reveal_folder};

    #[test]
    #[ignore = "M9 fill: a-files"]
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
    #[ignore = "M9 fill: a-files"]
    fn a_reveal_opens_the_parent_folder() {
        let cases: &[(&str, &str)] = &[
            ("/home/p/a.pdf", "/home/p"),
            ("/home/p/folder", "/home/p"),
            ("/", "/"),
        ];
        for &(path, want) in cases {
            assert_eq!(
                reveal_folder(Path::new(path)),
                PathBuf::from(want),
                "{path}"
            );
        }
    }
}
