//! What tests and demos drive surfaces with: fake services whose senders the test holds, and
//! [`SurfaceHarness`], any `Surface` mounted on them, and [`LockHarness`] for a `LockSurface`. Behind the `harness` feature; never part
//! of a production build.

pub mod fake;

mod ink;
mod lock;
mod surface;

pub use ds_blitz::FocusFallback;
pub use ink::ink_contrast;
pub use lock::{LockHarness, LockSetup};
pub use surface::{SurfaceHarness, SurfaceSetup};
