//! How large a user's picture is drawn (design/25-EMOJI.md section 7).

use ds::base::word::Word;

/// How large a picture is drawn (`data-size`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum PictureSize {
    /// 28 px: beside a name in a list or a menu.
    #[word(slug = "28")]
    Small,
    /// 64 px: a settings row, a switcher, the lock screen's prompt.
    #[word(slug = "64")]
    Medium,
    /// 128 px: a login screen's large picture.
    #[word(slug = "128")]
    Large,
}

impl PictureSize {
    /// The side in logical pixels.
    pub fn px(self) -> u16 {
        match self {
            PictureSize::Small => 28,
            PictureSize::Medium => 64,
            PictureSize::Large => 128,
        }
    }
}
