//! Launcher core: providers, queries, ranking and frecency, with no I/O (plan "Design:
//! `<shell>` repo": `sill-launcher`, future repo `sift`).
//!
//! Pure by construction: `scripts/check-boundary.sh` keeps `dioxus`, `zbus`, `wayland-client`,
//! `tokio` and `libpulse-binding` out of this crate. Providers that need the system live in
//! `sill-services::providers` and implement [`Provider`] from there; the clock is always an
//! argument ([`UnixTime`]).

pub mod activation;
pub mod frecency;
pub mod ids;
pub mod item;
pub mod matching;
pub mod provider;
pub mod query;
pub mod rank;
pub mod sink;
pub mod time;

pub use activation::{Activation, DesktopAction, SettingsTarget, SystemCommand, WindowKey};
pub use frecency::{Frecency, FrecencyScore, ItemKey, Use, age_bucket};
pub use ids::{ActionId, AppId, ItemId, ProviderKind};
pub use item::{Action, Choice, IconRef, Item};
pub use matching::{MatchScore, Needle};
pub use provider::Provider;
pub use query::{Cancel, Generation, Query};
pub use rank::{Prefix, RankWeights, Score, rank, rank_text};
pub use sink::{Batch, ResultSink, VecSink};
pub use time::UnixTime;
