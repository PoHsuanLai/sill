//! What is running: a `/proc` scan for this user's processes by name (cosmic-session,
//! cosmic-idle), under the locker's and idle's session resolution.

use std::os::unix::fs::MetadataExt;

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
