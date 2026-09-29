//! What every surface's document is rooted in: quire's faces and pre-paint pass, the seat's
//! signals bridge and [`SurfaceRoot`]. Every root gets quire's document host through
//! `ds_native::provide_host()`, called at the top of the root component.

mod host;
mod signals;
mod surface_root;

pub use host::{faces, host_config, prepaint};
pub use signals::HostSignalsBridge;
pub use surface_root::SurfaceRoot;
