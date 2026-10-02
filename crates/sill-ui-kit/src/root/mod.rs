//! What every surface's document is rooted in: quire's faces and pre-paint pass, the seat's
//! signals bridge and [`SurfaceRoot`]. Every root gets quire's document host through
//! `ds_blitz::provide_host()`, called at the top of the root component, and quire's
//! `HostSignals` through [`use_host_signals`], called by `mount`, `mount_lock` and `PopupFrame`.

mod click_away;
mod host;
mod scheme_keyed;
mod signals;
mod surface_root;
mod user_style;

pub use click_away::ClickAway;
pub use host::{faces, host_config, prepaint};
pub use scheme_keyed::SchemeKeyed;
pub use signals::{use_host_scale, use_host_signals};
pub use surface_root::SurfaceRoot;
pub use user_style::{RootStyle, SurfaceName, use_root_style};
