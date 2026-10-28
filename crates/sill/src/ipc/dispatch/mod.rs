//! The answer to each request: [`route`] is the one match over `Request`; `keys` and `debug` hold
//! the two arms that are more than a call. An arm calls a link, a service's control or a follower
//! and returns a `Response`; it holds no logic.

mod debug;
mod keys;
pub(crate) mod quit;
mod reach;
mod reply;
mod route;

pub use reach::{Carried, DesktopLine, Reach};
pub(crate) use reply::{failed, sent, unsupported};
pub use route::route;
