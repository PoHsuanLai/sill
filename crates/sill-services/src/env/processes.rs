//! What is running: this user's processes by name, which the
//! locker's and idle's session resolution reads. The daemon scans `/proc`; a test sees none.

use std::fmt;
use std::os::unix::fs::MetadataExt;

/// A process id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Pid(pub u32);

/// The processes a service can look for by name. Blocking: call it on the blocking pool.
pub trait Processes: Send + Sync + fmt::Debug {
    /// Processes of this user (other than this one) whose name is `name`.
    fn running(&self, name: &str) -> Vec<Pid>;
}

/// The real ones: a scan of `/proc`, matching `/proc/<pid>/comm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProcScan;

impl Processes for ProcScan {
    fn running(&self, name: &str) -> Vec<Pid> {
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
}

/// No process is running, whatever the name: what [`Env::isolated`](super::Env::isolated) sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoProcesses;

impl Processes for NoProcesses {
    fn running(&self, _name: &str) -> Vec<Pid> {
        Vec::new()
    }
}
