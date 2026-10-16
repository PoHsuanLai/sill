//! What every surface's document is rooted in: quire's faces and pre-paint pass, the seat's
//! signals bridge and [`SurfaceRoot`]. Every root gets quire's document host through
//! `ds_blitz::provide_host()` and the keymap through [`provide_keymap`], called at the top of the
//! root component, and quire's
//! `HostSignals` through [`use_host_signals`], called by `mount`, `mount_lock` and `PopupFrame`.

mod click_away;
mod file_roots;
mod host;
mod keymap;
mod signals;
mod surface_root;
mod user_style;

pub use click_away::ClickAway;
pub use file_roots::file_roots;
pub use host::{faces, host_config, prepaint};
pub use keymap::{desktop_keys, provide_keymap};
pub use signals::{use_host_scale, use_host_signals};
pub use surface_root::SurfaceRoot;
pub use user_style::{RootStyle, SurfaceName, use_root_style};
