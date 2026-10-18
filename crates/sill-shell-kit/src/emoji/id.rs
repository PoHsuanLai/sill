//! [`EmojiId`]: which emoji a user picked for their picture (user data), one of the shipped set
//! (design/25-EMOJI.md section 3). The slugs are the sheet files' stems and the serde names.

use super::sheet::SheetPx;
use ds::base::word::Word;
use serde::{Deserialize, Serialize};

macro_rules! emoji_set {
    ($(($variant:ident, $slug:literal, $text:literal, $name:literal)),+ $(,)?) => {
        /// One emoji of the shipped animated set. Serialised as its slug (`"heart-eyes"`), so a
        /// stored choice survives the set being reordered.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
        #[serde(rename_all = "kebab-case")]
        pub enum EmojiId {
            $(
                #[doc = $name]
                #[word(slug = $slug, label = $name)]
                $variant,
            )+
        }

        impl EmojiId {
            /// The emoji as text, for a picker's search or a plain-text fallback.
            pub fn text(self) -> &'static str {
                match self {
                    $(EmojiId::$variant => $text,)+
                }
            }

            /// The sheet PNG's bytes at `px`.
            pub(crate) fn png(self, px: SheetPx) -> &'static [u8] {
                match (self, px) {
                    $(
                        (EmojiId::$variant, SheetPx::Px128) =>
                            include_bytes!(concat!("../../assets/emoji/", $slug, "-128.png")),
                        (EmojiId::$variant, SheetPx::Px256) =>
                            include_bytes!(concat!("../../assets/emoji/", $slug, "-256.png")),
                    )+
                }
            }

            /// Its place in [`EmojiId::ALL`].
            pub(crate) fn index(self) -> usize {
                self as usize
            }
        }
    };
}

emoji_set![
    (Grinning, "grinning", "\u{1F600}", "Grinning face"),
    (
        GrinningEyes,
        "grinning-eyes",
        "\u{1F604}",
        "Grinning face with smiling eyes"
    ),
    (
        Beaming,
        "beaming",
        "\u{1F601}",
        "Beaming face with smiling eyes"
    ),
    (Laughing, "laughing", "\u{1F606}", "Grinning squinting face"),
    (
        GrinSweat,
        "grin-sweat",
        "\u{1F605}",
        "Grinning face with sweat"
    ),
    (Joy, "joy", "\u{1F602}", "Face with tears of joy"),
    (Rofl, "rofl", "\u{1F923}", "Rolling on the floor laughing"),
    (
        SlightSmile,
        "slight-smile",
        "\u{1F642}",
        "Slightly smiling face"
    ),
    (UpsideDown, "upside-down", "\u{1F643}", "Upside-down face"),
    (Wink, "wink", "\u{1F609}", "Winking face"),
    (
        Blush,
        "blush",
        "\u{1F60A}",
        "Smiling face with smiling eyes"
    ),
    (Halo, "halo", "\u{1F607}", "Smiling face with halo"),
    (Hearts, "hearts", "\u{1F970}", "Smiling face with hearts"),
    (
        HeartEyes,
        "heart-eyes",
        "\u{1F60D}",
        "Smiling face with heart-eyes"
    ),
    (StarStruck, "star-struck", "\u{1F929}", "Star-struck"),
    (
        KissingHeart,
        "kissing-heart",
        "\u{1F618}",
        "Face blowing a kiss"
    ),
    (Yum, "yum", "\u{1F60B}", "Face savoring food"),
    (
        WinkyTongue,
        "winky-tongue",
        "\u{1F61C}",
        "Winking face with tongue"
    ),
    (Zany, "zany", "\u{1F92A}", "Zany face"),
    (
        Hugging,
        "hugging",
        "\u{1F917}",
        "Smiling face with open hands"
    ),
    (
        Chuckling,
        "chuckling",
        "\u{1F92D}",
        "Face with hand over mouth"
    ),
    (Thinking, "thinking", "\u{1F914}", "Thinking face"),
    (
        RaisedEyebrow,
        "raised-eyebrow",
        "\u{1F928}",
        "Face with raised eyebrow"
    ),
    (Smirk, "smirk", "\u{1F60F}", "Smirking face"),
    (Relieved, "relieved", "\u{1F60C}", "Relieved face"),
    (Sleepy, "sleepy", "\u{1F62A}", "Sleepy face"),
    (Sleeping, "sleeping", "\u{1F634}", "Sleeping face"),
    (
        Sunglasses,
        "sunglasses",
        "\u{1F60E}",
        "Smiling face with sunglasses"
    ),
    (Nerd, "nerd", "\u{1F913}", "Nerd face"),
    (Partying, "partying", "\u{1F973}", "Partying face"),
    (Cowboy, "cowboy", "\u{1F920}", "Cowboy hat face"),
    (MindBlown, "mind-blown", "\u{1F92F}", "Exploding head"),
    (Confounded, "confounded", "\u{1F616}", "Confounded face"),
    (Persevering, "persevering", "\u{1F623}", "Persevering face"),
    (Eyes, "eyes", "\u{1F440}", "Eyes"),
    (Wave, "wave", "\u{1F44B}", "Waving hand"),
    (ThumbsUp, "thumbs-up", "\u{1F44D}", "Thumbs up"),
    (RaisingHands, "raising-hands", "\u{1F64C}", "Raising hands"),
    (Victory, "victory", "\u{270C}\u{FE0F}", "Victory hand"),
    (Ghost, "ghost", "\u{1F47B}", "Ghost"),
    (Robot, "robot", "\u{1F916}", "Robot"),
    (Fox, "fox", "\u{1F98A}", "Fox"),
];

impl Default for EmojiId {
    /// A smiling face: warm, and neither the unlock nor the wrong-password reaction.
    fn default() -> Self {
        EmojiId::Blush
    }
}
