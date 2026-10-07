//! The optional disc behind an animated emoji, and whether it plays. The disc: none, or a
//! tinted circle on one of the icon palette's eight hues (design/08 section 2.10), pale on light
//! and deep on dark.

use ds::base::colour::fit::oklch_hex;
use ds::base::word::Word;
use ds::style::appearance::theme::Scheme;
use serde::{Deserialize, Serialize};

/// One of the icon palette's eight hues, for a tinted disc. Stored by its name (`"teal"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscHue {
    /// Hue 40.
    Clay,
    /// Hue 85.
    Ochre,
    /// Hue 130.
    Sage,
    /// Hue 175.
    Jade,
    /// Hue 220.
    Teal,
    /// Hue 265.
    Slate,
    /// Hue 310.
    Plum,
    /// Hue 355.
    Rose,
}

/// What sits behind the emoji (`data-disc`). User data beside the [`super::EmojiId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmojiDisc {
    /// The emoji alone, at the full size of its box.
    #[default]
    None,
    /// A tinted circle, the emoji inset on it.
    Tinted(DiscHue),
}

impl EmojiDisc {
    /// The `data-disc` value.
    pub(crate) fn slug(self) -> &'static str {
        match self {
            EmojiDisc::None => "none",
            EmojiDisc::Tinted(_) => "tinted",
        }
    }
}

fn hue(disc: DiscHue) -> f64 {
    match disc {
        DiscHue::Clay => 40.0,
        DiscHue::Ochre => 85.0,
        DiscHue::Sage => 130.0,
        DiscHue::Jade => 175.0,
        DiscHue::Teal => 220.0,
        DiscHue::Slate => 265.0,
        DiscHue::Plum => 310.0,
        DiscHue::Rose => 355.0,
    }
}

/// The disc's colour as `#rrggbb` in `scheme`.
pub(crate) fn tint(disc: DiscHue, scheme: Scheme) -> String {
    match scheme {
        Scheme::Light => oklch_hex(0.91, 0.055, hue(disc)),
        Scheme::Dark => oklch_hex(0.40, 0.06, hue(disc)),
    }
}

/// Whether a picture plays at all (`data-playback`). A picker showing the whole set passes
/// `Still`: 42 loops at once is motion nobody asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum EmojiPlayback {
    /// Plays its own animation once through when it appears and on each wake.
    #[default]
    Once,
    /// The rest frame only, as under Reduced motion.
    Still,
}
