//! What the daemon asks of the kernel before it holds a password: no core file. The lock screen's
//! password field lives in this process's memory for as long as it is typed, and a crash would
//! otherwise write that memory to a file other programs of the user, or a crash collector, keep.
//!
//! The process is left dumpable on purpose: marking it not dumpable makes `/proc/<pid>/exe` and
//! `/proc/<pid>/root` unreadable to the user's other programs, which the portals and the
//! identity checks of the desktop's daemons read to tell who is calling.

use rustix::process::{Resource, Rlimit, setrlimit};

/// Forbid core files for this process and its children.
pub(super) fn forbid_core_files() {
    let none = Rlimit {
        current: Some(0),
        maximum: Some(0),
    };
    if let Err(error) = setrlimit(Resource::Core, none) {
        eprintln!("sill: could not forbid core files: {error}");
    }
}

#[cfg(test)]
mod tests {
    use rustix::process::{Resource, getrlimit};

    use super::forbid_core_files;

    /// The limit is process-wide, and lowering it is harmless to the other tests (none write core
    /// files).
    #[test]
    fn core_files_are_forbidden_after_the_call() {
        forbid_core_files();
        let limit = getrlimit(Resource::Core);
        assert_eq!(limit.current, Some(0));
        assert_eq!(limit.maximum, Some(0));
    }
}
