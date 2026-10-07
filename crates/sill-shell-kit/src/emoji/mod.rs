//! Animated emoji: the user's picture as an emoji they pick, moving (design/25-EMOJI.md).
//! The frames are Noto Animated Emoji (CC BY 4.0, `assets/emoji/ATTRIBUTION.txt`),
//! pre-rendered into sprite sheets by `tools/emoji`, because the renderer draws one frame of an
//! image and plays no Lottie; quire plays them by moving the sheet's `background-position`
//! from a Rust timer, once through when the emoji appears (design/30 section 2.10: the asset's
//! own animation, no loop of quire's).

use ds::base::word::Word;
use {
    disc::{EmojiDisc, EmojiPlayback},
    id::EmojiId,
};

pub mod disc;
pub mod id;
pub(crate) mod play;
pub(crate) mod sheet;
#[cfg(test)]
mod tests;

use crate::user_picture::size::PictureSize;
use dioxus::prelude::*;
use ds::motion::wake::WakeStamp;
use ds::root::common::Common;
use ds::style::scope::use_scope;
use sheet::{SheetPx, position, timing, uri};

/// The attribution a surface that shows these emoji carries in its credits (design/25
/// section 2).
pub const EMOJI_ATTRIBUTION: &str = "Animated emoji: Noto Animated Emoji by Google, \
    CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/); frames resampled and packed.";

/// The user's `emoji` at `size`, playing its own animation once through when it mounts, on each
/// new `wake` stamp and on each new pick, then resting on its first frame and painting nothing
/// (design/25-EMOJI.md section 5). Under Reduced motion, or with `playback:
/// EmojiPlayback::Still`, only the still frame is shown. Decorative: the name beside it is what a
/// screen reader reads.
#[component]
pub fn AnimatedEmoji(
    emoji: EmojiId,
    size: PictureSize,
    #[props(default)] wake: WakeStamp,
    #[props(default)] disc: EmojiDisc,
    #[props(default)] playback: EmojiPlayback,
    #[props(default)] common: Common,
) -> Element {
    let scheme = use_scope().scheme;
    let frame = play::use_frame(emoji, wake, playback);
    let px = SheetPx::for_size(size);
    let (at, fit) = position(timing(emoji), frame);
    let sheet = format!("url(\"{}\")", uri(emoji, px));
    let tint = match disc {
        EmojiDisc::Tinted(hue) => Some(format!("--em-disc:{}", disc::tint(hue, scheme))),
        EmojiDisc::None => None,
    };
    let class = common.class("ds-emoji");
    let data = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "data-size": size.slug(),
            "data-disc": disc.slug(),
            "data-playback": playback.slug(),
            "aria-hidden": "true",
            style: tint,
            onmounted: move |event| common.mounted(event),
            ..data,
            div {
                class: "ds-emoji-face",
                "data-emoji": emoji.slug(),
                "data-frame": "{frame}",
                background_image: sheet,
                background_size: fit,
                background_position: at,
            }
        }
    }
}
