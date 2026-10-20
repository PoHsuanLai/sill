//! Serving a well-known bus name: whether the shell holds it, and why not. One vocabulary for
//! every service that claims a name (the work areas, the workspaces, the accounts sheet, the
//! seat at intentd), so none of them spells "not told yet / serving / could not" its own way.

/// Why a bus name is not served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimError {
    /// There is no bus to serve on.
    NoBus,
    /// Another process owns a name the service needs; the text says which.
    NameTaken(String),
    /// The bus refused something else; its words.
    Refused(String),
}

/// Whether a service holds its bus name. A service that cannot says why, and never sits at its
/// default pretending.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ServeLink {
    /// Not claiming names (a test's `start`, or before the service has tried): nothing touches a
    /// bus.
    #[default]
    Passive,
    /// The name is ours.
    Serving,
    /// It could not be taken.
    Unavailable(ClaimError),
}
