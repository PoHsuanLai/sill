//! The answer to each request, one dispatch fn per request group ([`route`] picks it). An arm
//! calls a link, a service's control or a follower and returns a `Response`; it holds no logic.

mod act;
mod debug;
mod keys;
pub(crate) mod quit;
mod reach;
mod reply;
mod route;
mod show;
mod status;
mod switcher;
mod workspace;

pub use reach::{Carried, DesktopLine, Reach};
pub(crate) use reply::sent;
pub use route::route;
pub(crate) use show::focused_or_first;
