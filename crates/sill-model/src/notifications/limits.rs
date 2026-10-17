//! How much of a notification the shell holds. An app chooses every field of a `Notify` call, so
//! each has a ceiling; the D-Bus boundary cuts to these (`sill_services::notifications`) and the
//! reducer keeps the live banners inside them. A sender that goes past one is cut or refused,
//! never allowed to grow what the shell holds without bound.

/// The summary: one line on a card.
pub const SUMMARY_MAX_BYTES: usize = 256;

/// The body, before its markup is read: a card shows a few lines, and the history keeps a hundred.
pub const BODY_MAX_BYTES: usize = 4096;

/// The app's name as it heads a card.
pub const APP_NAME_MAX_BYTES: usize = 128;

/// How many actions (key and label pairs) one notification offers: a card has room for a few
/// buttons.
pub const ACTIONS_MAX: usize = 8;

/// An action's key, which comes back in `ActionInvoked`.
pub const ACTION_KEY_MAX_BYTES: usize = 128;

/// An action's label: it is drawn on a button.
pub const ACTION_LABEL_MAX_BYTES: usize = 64;

/// How many hints one call may carry: the spec names about fifteen, and the rest are ignored.
pub const HINTS_MAX: usize = 32;

/// A name or word in a hint (`category`, `desktop-entry`, `sound-name`).
pub const HINT_TEXT_MAX_BYTES: usize = 256;

/// A path or icon reference in an argument or a hint.
pub const HINT_PATH_MAX_BYTES: usize = 1024;

/// The longest side of `image-data` in pixels: a card draws it at a fraction of this.
pub const IMAGE_MAX_SIDE: u32 = 256;

/// The most bytes of `image-data`'s array the boundary reads: a square at [`IMAGE_MAX_SIDE`] with
/// four channels and padded rows fits.
pub const IMAGE_MAX_BYTES: usize = 1 << 20;

/// Room for what a `Notify` body holds besides the values the ceilings above cut: length prefixes,
/// padding to each value's alignment, the variants' signatures and the image's six numbers. A
/// call with thirty-two hints needs about two kilobytes of it; this is eight times that.
const NOTIFY_CALL_MARGIN_BYTES: usize = 16 * 1024;

/// The most bytes a `Notify` body may hold before the server refuses it unread: every argument
/// at its ceiling (the app's name and icon, the summary, the body, [`ACTIONS_MAX`] key and label
/// pairs, [`HINTS_MAX`] hints each with a word and a path, and one image's array) plus
/// [`NOTIFY_CALL_MARGIN_BYTES`]. A call within it is read and each field cut to its own ceiling;
/// a call over it cannot be a notification an app meant to post.
pub const NOTIFY_CALL_MAX_BYTES: usize = APP_NAME_MAX_BYTES
    + HINT_PATH_MAX_BYTES
    + SUMMARY_MAX_BYTES
    + BODY_MAX_BYTES
    + ACTIONS_MAX * (ACTION_KEY_MAX_BYTES + ACTION_LABEL_MAX_BYTES)
    + HINTS_MAX * (HINT_TEXT_MAX_BYTES + HINT_PATH_MAX_BYTES)
    + IMAGE_MAX_BYTES
    + NOTIFY_CALL_MARGIN_BYTES;

/// A link's target: a longer one is no link.
pub const HREF_MAX_BYTES: usize = 2048;

/// How many styled runs one body holds: the rest of the text joins the last run.
pub const BODY_RUNS_MAX: usize = 128;

/// How many links may be open inside each other: a link inside a link keeps the outer target.
pub const LINK_NESTING_MAX: usize = 1;

/// How many banners are on screen or waiting at once, whoever sent them.
pub const LIVE_BANNERS_MAX: usize = 50;

/// How many banners one app may have at once: a flood from one app does not push the others off.
pub const BANNERS_PER_APP_MAX: usize = 10;

/// How long an app's critical banner stays when nobody dismisses it: long enough to be seen on
/// return from a break, short enough that a forgotten one is not there next week.
pub const APP_CRITICAL_HOLD_MS: i64 = 10 * 60 * 1000;

/// How many notifications one sender may post in a burst.
pub const SENDER_BURST: u32 = 30;

/// How often a sender earns one more notification after a burst.
pub const SENDER_REFILL_MS: i64 = 250;

/// How many notifications all callers together may post in a burst: more than one caller's, so
/// several chatty apps pass, and fewer than a flood from many connections would be.
pub const ALL_SENDERS_BURST: u32 = 120;

/// How often the callers together earn one more notification after a burst (ten a second).
pub const ALL_SENDERS_REFILL_MS: i64 = 100;

/// How many senders the rate limiter tracks at once.
pub const SENDERS_TRACKED_MAX: usize = 256;
