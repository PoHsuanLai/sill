//! Notifications: what apps post, the banners and the history, the policy that presents them, and
//! the step over every event.

mod capabilities;
mod command;
mod content;
mod ids;
mod markup;
mod markup_parse;
mod model;
mod policy;
mod rules;
mod step;
mod stored;
#[cfg(test)]
mod tests;

pub use capabilities::{CAPABILITIES, SERVER_INFORMATION, SPEC_VERSION, ServerInformation};
pub use command::{
    Clear, CloseReason, NotificationsCommand, NotificationsEffect, NotificationsEvent,
};
pub use content::{
    Action, AppRef, Expiry, Hints, IconRef, NoteImage, Notification, NotifyUrgency, Pixels,
    Posting, Residency, SoundHint, Transience, pair_actions,
};
pub use ids::{ActionKey, Activation, ActivationToken, AppKey, NotificationId, UnixMs};
pub use markup::{Body, BodyRun, Href, RunLink, RunStyle, Slant, Underline, Weight, parse_body};
pub use model::{
    AppGroup, BannerLife, History, LiveBanner, NameHolder, NameOwner, NotesState, Notifications,
    ServerMode, ServerStatus,
};
pub use policy::{
    Chime, HistoryCap, NotificationsPolicy, Presentation, Presented, Quiet, Swipe, chime, presented,
};
pub use rules::holds;
pub use step::{assign_id, step};
pub use stored::{STORED_VERSION, StoredHistory, for_disk};
