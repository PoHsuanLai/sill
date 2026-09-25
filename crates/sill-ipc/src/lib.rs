//! The sill IPC protocol: newline-delimited JSON requests and responses, one request per
//! connection (plan "Design: `<shell>` repo", CLI/IPC).
//!
//! Pure: no socket, no runtime. The daemon (`sill::ipc_server`) and the command line
//! (`sill::cli`) own the stream; this crate owns the words on it.

pub mod error;
pub mod line;
pub mod request;
pub mod response;

pub use error::IpcError;
pub use line::{MAX_LINE, decode, encode, read_message, write_message};
pub use request::{
    ControlCenterRequest, DebugRequest, DesktopId, DisplayRequest, DockRequest, FakeAction,
    FakeExpire, FakeMute, FakeNotification, FakeOsd, FakeUrgency, LauncherRequest, LevelRequest,
    NotificationsRequest, OsdTopic, Point, PointerButton, PowerMenuRequest, Request, SurfaceName,
    VolumeRequest,
};
pub use response::{
    Failure, FailureKind, Micros, NotificationsReport, ReportHolder, ReportMode, Response,
    SurfaceStats,
};
