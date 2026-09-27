//! The sill IPC protocol: newline-delimited JSON requests and responses, one request per
//! connection (plan "Design: `<shell>` repo", CLI/IPC).
//!
//! Pure: no socket, no runtime. The daemon (`sill::ipc_server`) and the command line
//! (`sill::cli`) own the stream; this crate owns the words on it.

pub mod desktop;
pub mod error;
pub mod idle;
pub mod launcher;
pub mod line;
pub mod request;
pub mod response;
pub mod session;

pub use desktop::{
    CornerName, DragPhase, FakeCorner, FakeScreenshot, FakeWidgetDrag, ScreenshotRequest, ShotArea,
    WidgetsRequest,
};
pub use error::IpcError;
pub use idle::{
    IdleReport, IdleRequest, ReportDim, ReportIdleMode, ReportIdlePhase, ReportInhibition,
    ReportLockCause, ReportOffReason, ReportPower,
};
pub use launcher::{
    ClipboardReport, ClipboardRequest, FakeLauncherKey, LauncherRows, ReportClipMode,
    ReportClipProtocol, ReportPreview, ReportPreviewKind, RowsSection,
};
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
pub use session::{
    FakeAuth, FakePolkit, FakeSwitcher, LockReport, LockRequest, PolkitReport, PolkitRequest,
    ReportDrawer, ReportLockPhase, ReportRegistration, SwitcherRequest,
};
