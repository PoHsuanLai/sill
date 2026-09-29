//! The answer to each request, one dispatch fn per request group ([`route`] picks it). An arm
//! calls a link, a service's control or a follower and returns a `Response`; it holds no logic.

mod act;
mod debug;
mod keys;
mod reach;
mod reply;
mod route;
mod show;
mod status;
mod switcher;

pub use reach::Reach;
pub use route::route;
