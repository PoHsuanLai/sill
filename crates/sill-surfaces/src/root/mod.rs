//! What every surface's document is rooted in: the host seams, the seat's modality bridge and
//! [`SurfaceRoot`].

mod host;
mod modality;
mod surface_root;

pub use host::{CARET, FOCUS, MEASURE, faces, host_config, prepaint};
pub use modality::HostModalityBridge;
pub use surface_root::SurfaceRoot;
