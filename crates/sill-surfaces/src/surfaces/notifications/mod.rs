//! Notification banners (design/13 §13.3.6): not a milestone M1-M4 surface; the banner machine
//! is frozen now (design/13 §13.4 places it at `surfaces/notifications/banner.rs`).

pub mod banner;

pub use banner::{
    Banner, BannerIn, BannerOut, BannerParams, Deadline, Leave, Persistence, Remaining, step,
};
