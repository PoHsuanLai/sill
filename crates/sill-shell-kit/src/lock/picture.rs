//! The person's picture in the lock and polkit prompts (design/30 section 2.10): a face
//! redrawn at the prompt's size, a photo cropped round at it, or an animated emoji playing its
//! own animation once as the prompt appears.

use crate::user_picture::draw::Sizes;
use crate::user_picture::size::PictureSize;
use ds::components::content::avatar::AvatarSize;

/// The lock screen's: 64 for all three.
pub(crate) const AT_LOCK: Sizes = Sizes {
    face: AvatarSize::Size64,
    photo: 64,
    emoji: PictureSize::Medium,
};

/// The polkit sheet's: 48. The emoji is drawn at `Medium` and `polkit_prompt.css` sets its box
/// to 48 (it is drawn in shares of its box, so it scales whole).
pub(crate) const AT_POLKIT: Sizes = Sizes {
    face: AvatarSize::Size48,
    photo: 48,
    emoji: PictureSize::Medium,
};
