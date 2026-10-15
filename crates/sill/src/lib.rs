//! The sill daemon and command line, as a library so `crates/sill/tests/**` can exercise its
//! internals (`main.rs` is a thin wrapper over this crate).

pub mod cli;
pub mod companion;
pub mod corners;
pub mod daemon;
pub mod edit_widgets;
pub mod eyedropper;
pub mod idle_dim;
pub mod intents;
pub mod ipc;
pub mod launch;
pub mod lock;
pub mod notifications;
pub mod osd;
pub mod outputs;
pub mod policy;
pub mod run_args;
pub mod scroll;
pub mod spaces_bar;
pub mod split_picker;
pub mod voice;
pub mod window_menu;
pub mod work_areas;
pub mod workspace_looks;
