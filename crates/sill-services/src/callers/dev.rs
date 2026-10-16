//! A person's own rows (feature `dev-callers`, which only the sill binary's `debug` feature turns
//! on): the stand-ins a checkout builds, named in a file the person owns, are let in as the
//! services they stand for, at `Proof::Dev`, below every real proof. A release build has none of
//! this: the feature, the flag and peephole's `dev-rows` are all absent.

use std::path::Path;

use peephole_bus::peephole::{LoadError, RealProc, TableFiles};
use peephole_bus::peephole_core::{Table, Uid};

use super::DesktopCallers;

impl DesktopCallers {
    /// These callers, with the developer rows in `path` added after the shipped ones (a row never
    /// takes a unit the shipped table names). The file must be a plain file of this user's (or
    /// root's) that no one else can write; otherwise nothing is added and the error says why.
    pub fn with_dev_rows(self, path: &Path) -> Result<DesktopCallers, LoadError> {
        let person = Uid::new(rustix::process::getuid().as_raw());
        let rows = TableFiles::load_dev(&RealProc::system(), path, person)?;
        Ok(self.with_rows(rows))
    }

    /// These callers, with `rows` merged after the shipped ones.
    pub fn with_rows(self, rows: Table) -> DesktopCallers {
        DesktopCallers::new(self.table.merge(rows))
    }
}
