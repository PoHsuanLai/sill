//! Launcher core: providers, queries, ranking and frecency, with no I/O (future repo `sift`).
//!
//! Pure by construction: `scripts/check-boundary.sh` keeps `dioxus`, `zbus`, `wayland-client`,
//! `tokio` and `libpulse-binding` out of this crate. Providers that need the system live in
//! `sill-services::providers` and implement [`Provider`] from there; the clock is always an
//! argument ([`UnixTime`]).

pub mod activation;
pub mod chord;
pub mod frecency;
pub mod ids;
pub mod item;
pub mod matching;
pub mod preview;
pub mod provider;
pub mod query;
pub mod rank;
pub mod sink;
pub mod subject;
pub mod time;

pub use activation::{
    Activation, DesktopAction, OpenTarget, SettingsTarget, SystemCommand, WindowKey,
};
pub use chord::{ActionKeys, Chord, ChordKey, Modifier, action_for_chord, chords};
pub use frecency::{Counting, Frecency, FrecencyScore, ItemKey, Use, age_bucket, counting};
pub use ids::{ActionId, AppId, ClipKey, ItemId, ProviderKind};
pub use item::{Action, Choice, IconRef, Item};
pub use matching::{MatchScore, Needle};
pub use preview::{ExcerptLimit, PageIndex, Preview, PreviewKind, excerpt_of, preview_for};
pub use provider::Provider;
pub use query::{Cancel, Generation, Query, Scope};
pub use rank::sections::{
    Cap, Expanded, More, Placement, ProviderRule, QueryShape, Section, SectionPlan, SectionSpan,
    ShownSection, Weight, place, shown, weighted,
};
pub use rank::{Prefix, RankWeights, Score, rank, rank_text};
pub use sink::{Batch, ResultSink, VecSink};
pub use subject::{
    CharCount, ClipFacts, ClipKind, EmojiFacts, Excerpt, FileFacts, Mime, SkinTone, Subject,
    WebFacts,
};
pub use time::UnixTime;
