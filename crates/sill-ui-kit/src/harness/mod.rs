//! What tests and demos drive surfaces with: fake services whose senders the test holds, and
//! [`SurfaceHarness`], any `Surface` mounted on them. Behind the `harness` feature; never part
//! of a production build.

pub mod fake;

mod surface;

pub use surface::{SurfaceHarness, SurfaceSetup};
