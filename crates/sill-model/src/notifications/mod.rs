//! Notifications: what apps post, the banners and the history, the policy that presents them, and
//! the step over every event.

mod capabilities;
mod command;
mod content;
mod ids;
mod limits;
#[cfg(test)]
mod limits_tests;
mod markup;
mod markup_parse;
mod model;
mod origin;
mod policy;
mod rate;
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
pub use limits::{
    ACTION_KEY_MAX_BYTES, ACTION_LABEL_MAX_BYTES, ACTIONS_MAX, APP_CRITICAL_HOLD_MS,
    APP_NAME_MAX_BYTES, BANNERS_PER_APP_MAX, BODY_MAX_BYTES, HINT_PATH_MAX_BYTES,
    HINT_TEXT_MAX_BYTES, HINTS_MAX, IMAGE_MAX_BYTES, IMAGE_MAX_SIDE, LIVE_BANNERS_MAX,
    SENDER_BURST, SENDERS_TRACKED_MAX, SUMMARY_MAX_BYTES,
};
pub use markup::{Body, BodyRun, Href, RunLink, RunStyle, Slant, Underline, Weight, parse_body};
pub use model::{
    AppGroup, BannerLife, History, LiveBanner, NameHolder, NameOwner, NotesState, Notifications,
    ServerMode, ServerStatus,
};
pub use origin::app_entry;
pub use policy::{
    Chime, HistoryCap, NotificationsPolicy, Presentation, Presented, Quiet, Swipe, chime, presented,
};
pub use rate::Allowance;
pub use rules::holds;
pub use step::{assign_id, step};
pub use stored::{STORED_VERSION, StoredHistory, for_disk};
