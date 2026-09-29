//! The clipboard history: entries, what is kept and what is skipped, and the pure step over copies
//! and policy.

mod command;
mod model;
mod offer;
mod step;

pub use command::{ClipEffect, ClipEvent, ClipboardCommand, Skipped};
pub use model::{
    ClipContent, ClipEntry, ClipImage, ClipPolicy, ClipProtocol, ClipState, ClipStatus, ClipText,
    Clipboard, EntryCap, HistoryMode, ImageCap,
};
pub use offer::{ImagesKept, MimePick, OfferVerdict, SECRET_HINTS, offer_verdict};
pub use step::step;
