//! What is installed and what is running: `PATH` lookups and a `/proc` scan, the effects under
//! the locker's and the polkit agent's pure resolution tables.

use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::polkit::Place;

/// `name` on `PATH`, as an executable file.
pub fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| executable(candidate).is_some())
}

/// `place`, if a program is there.
pub fn find(place: Place) -> Option<PathBuf> {
    match place {
        Place::At(path) => executable(Path::new(path)).map(|()| PathBuf::from(path)),
        Place::OnPath(name) => on_path(name),
    }
}

fn executable(path: &Path) -> Option<()> {
    let meta = std::fs::metadata(path).ok()?;
    (meta.is_file() && meta.permissions().mode() & 0o111 != 0).then_some(())
}

/// A process id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Pid(pub u32);

/// Processes of this user (other than this one) whose name (`/proc/<pid>/comm`) is `name`.
pub fn running(name: &str) -> Vec<Pid> {
    let Some(me) = std::fs::metadata("/proc/self").ok().map(|m| m.uid()) else {
        return Vec::new();
    };
    let own = std::process::id();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let pid = entry.file_name().to_str()?.parse::<u32>().ok()?;
            let ours = entry.metadata().ok()?.uid() == me && pid != own;
            let comm = std::fs::read_to_string(entry.path().join("comm")).ok()?;
            (ours && comm.trim_end() == name).then_some(Pid(pid))
        })
        .collect()
}

/// Whether `pid` still names a live process (a zombie counts as gone).
pub fn still_running(pid: Pid) -> Option<Pid> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid.0)).ok()?;
    // `pid (comm) S ...`: the state letter follows the last `)`.
    let state = stat.rsplit_once(')')?.1.split_whitespace().next()?;
    (state != "Z" && state != "X").then_some(pid)
}
