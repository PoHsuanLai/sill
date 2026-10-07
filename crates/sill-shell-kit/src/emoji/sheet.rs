//! The shipped sprite sheets and their manifest (design/25-EMOJI.md section 4): which PNG an
//! emoji draws from at a size, its grid, and how long each frame is held.
//!
//! The sheets are `tools/emoji`'s output under `assets/emoji/`, compiled in with
//! `include_bytes!` and handed to the renderer as `data:` URIs, as the grain tile is: a quire
//! document fetches nothing.

use super::id::EmojiId;
use crate::user_picture::size::PictureSize;
use ds::base::word::Word;
use ds::style::icon::url::IconUrl;
use serde::Deserialize;
use std::sync::{LazyLock, OnceLock};
use std::time::Duration;

/// The two frame sizes a sheet is drawn at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SheetPx {
    /// 128 px frames: 28 and 64 px pictures, at up to 2x.
    Px128,
    /// 256 px frames: the 128 px picture at 2x.
    Px256,
}

impl SheetPx {
    /// The sheet a picture of `size` reads: always enough pixels for a 2x display.
    pub(crate) fn for_size(size: PictureSize) -> SheetPx {
        match size {
            PictureSize::Small | PictureSize::Medium => SheetPx::Px128,
            PictureSize::Large => SheetPx::Px256,
        }
    }

    /// Pixels on a frame's side.
    #[cfg(test)]
    pub(crate) fn px(self) -> u32 {
        match self {
            SheetPx::Px128 => 128,
            SheetPx::Px256 => 256,
        }
    }
}

macro_rules! sheets {
    ($(($variant:ident, $slug:literal)),+ $(,)?) => {
        /// The sheet PNG's bytes.
        pub(crate) fn png(emoji: EmojiId, px: SheetPx) -> &'static [u8] {
            match (emoji, px) {
                $(
                    (EmojiId::$variant, SheetPx::Px128) =>
                        include_bytes!(concat!("../../assets/emoji/", $slug, "-128.png")),
                    (EmojiId::$variant, SheetPx::Px256) =>
                        include_bytes!(concat!("../../assets/emoji/", $slug, "-256.png")),
                )+
            }
        }
    };
}

sheets![
    (Grinning, "grinning"),
    (GrinningEyes, "grinning-eyes"),
    (Beaming, "beaming"),
    (Laughing, "laughing"),
    (GrinSweat, "grin-sweat"),
    (Joy, "joy"),
    (Rofl, "rofl"),
    (SlightSmile, "slight-smile"),
    (UpsideDown, "upside-down"),
    (Wink, "wink"),
    (Blush, "blush"),
    (Halo, "halo"),
    (Hearts, "hearts"),
    (HeartEyes, "heart-eyes"),
    (StarStruck, "star-struck"),
    (KissingHeart, "kissing-heart"),
    (Yum, "yum"),
    (WinkyTongue, "winky-tongue"),
    (Zany, "zany"),
    (Hugging, "hugging"),
    (Chuckling, "chuckling"),
    (Thinking, "thinking"),
    (RaisedEyebrow, "raised-eyebrow"),
    (Smirk, "smirk"),
    (Relieved, "relieved"),
    (Sleepy, "sleepy"),
    (Sleeping, "sleeping"),
    (Sunglasses, "sunglasses"),
    (Nerd, "nerd"),
    (Partying, "partying"),
    (Cowboy, "cowboy"),
    (MindBlown, "mind-blown"),
    (Confounded, "confounded"),
    (Persevering, "persevering"),
    (Eyes, "eyes"),
    (Wave, "wave"),
    (ThumbsUp, "thumbs-up"),
    (RaisingHands, "raising-hands"),
    (Victory, "victory"),
    (Ghost, "ghost"),
    (Robot, "robot"),
    (Fox, "fox"),
];

/// `manifest.json` as `tools/emoji` writes it; only what playback reads.
#[derive(Debug, Deserialize)]
pub(crate) struct Manifest {
    pub(crate) emoji: Vec<Entry>,
}

/// One emoji's sheet layout and timing.
#[derive(Debug, Deserialize)]
pub(crate) struct Entry {
    pub(crate) slug: String,
    pub(crate) columns: u16,
    pub(crate) rows: u16,
    pub(crate) durations_ms: Vec<u16>,
}

pub(crate) const MANIFEST_JSON: &str = include_str!("../../assets/emoji/manifest.json");

/// The manifest, parsed once. A manifest that does not parse is a broken build of
/// `tools/emoji`, caught by this module's tests; at run time it plays nothing.
static MANIFEST: LazyLock<Manifest> =
    LazyLock::new(|| serde_json::from_str(MANIFEST_JSON).unwrap_or(Manifest { emoji: Vec::new() }));

/// How an emoji's frames are laid out and timed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timing {
    /// Frames across a sheet row.
    pub(crate) columns: u16,
    /// Rows of frames.
    pub(crate) rows: u16,
    /// Frames in the loop; frame 0 is the rest pose.
    pub(crate) frames: u16,
}

/// A still, one-frame timing for an emoji the manifest does not list.
const STILL: Timing = Timing {
    columns: 1,
    rows: 1,
    frames: 1,
};

fn entry(emoji: EmojiId) -> Option<&'static Entry> {
    MANIFEST
        .emoji
        .get(emoji.index())
        .filter(|entry| entry.slug == emoji.slug())
        .or_else(|| {
            MANIFEST
                .emoji
                .iter()
                .find(|entry| entry.slug == emoji.slug())
        })
}

/// `emoji`'s layout.
pub(crate) fn timing(emoji: EmojiId) -> Timing {
    entry(emoji).map_or(STILL, |entry| Timing {
        columns: entry.columns.max(1),
        rows: entry.rows.max(1),
        frames: u16::try_from(entry.durations_ms.len())
            .unwrap_or(u16::MAX)
            .max(1),
    })
}

/// How long `emoji` holds each frame, in loop order.
pub(crate) fn durations(emoji: EmojiId) -> Vec<Duration> {
    entry(emoji).map_or_else(Vec::new, |entry| {
        entry
            .durations_ms
            .iter()
            .map(|&ms| Duration::from_millis(u64::from(ms)))
            .collect()
    })
}

const COUNT: usize = EmojiId::ALL.len();
static URIS_128: [OnceLock<String>; COUNT] = [const { OnceLock::new() }; COUNT];
static URIS_256: [OnceLock<String>; COUNT] = [const { OnceLock::new() }; COUNT];

/// The sheet as a `data:image/png;base64,…` URI, encoded once per process.
pub(crate) fn uri(emoji: EmojiId, px: SheetPx) -> &'static str {
    let cache = match px {
        SheetPx::Px128 => &URIS_128,
        SheetPx::Px256 => &URIS_256,
    };
    cache[emoji.index()].get_or_init(|| IconUrl::png(png(emoji, px)).as_str().to_owned())
}

/// Where frame `frame` sits, as a `background-position` in shares of the box, and the
/// `background-size` that makes one frame fill it: so the same sheet fits 28, 64 or 128 px.
pub(crate) fn position(timing: Timing, frame: u16) -> (String, String) {
    let frame = frame % timing.frames.max(1);
    let column = frame % timing.columns;
    let row = frame / timing.columns;
    let share = |at: u16, of: u16| match of {
        0 | 1 => 0.0,
        _ => f64::from(at) * 100.0 / f64::from(of - 1),
    };
    let x = share(column, timing.columns);
    let y = share(row, timing.rows);
    (
        format!("{}% {}%", trim(x), trim(y)),
        format!(
            "{}% {}%",
            u32::from(timing.columns) * 100,
            u32::from(timing.rows) * 100
        ),
    )
}

/// A share with at most three decimals and no trailing zeros.
fn trim(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    text.to_owned()
}
