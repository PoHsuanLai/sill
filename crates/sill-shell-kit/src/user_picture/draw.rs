//! Drawing a [`UserPicture`] (design/30 section 2.10): one wrapper (`div.ds-user-picture`) around
//! a face, a photo or an animated emoji, at the size the surface gives each kind. The lock and
//! polkit prompts draw through [`drawn`]; a face is `Avatar`'s own disc.

use super::picture::UserPicture;
use super::size::PictureSize;
use crate::emoji::AnimatedEmoji;
use dioxus::prelude::*;
use ds::components::content::avatar::{AvatarFace, AvatarSize, face};
use ds::components::content::image_source::ImageSource;

/// How large each kind of picture is drawn at one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sizes {
    pub(crate) face: AvatarSize,
    pub(crate) photo: u16,
    pub(crate) emoji: PictureSize,
}

/// `picture` at `sizes`, in its wrapper.
pub(crate) fn drawn(picture: UserPicture, sizes: Sizes) -> Element {
    let body = match picture {
        UserPicture::Face(avatar) => face(AvatarFace {
            size: sizes.face,
            ..avatar
        }),
        UserPicture::Photo(source) => photo(source, sizes.photo),
        UserPicture::Emoji(emoji) => rsx! {
            AnimatedEmoji { emoji, size: sizes.emoji }
        },
    };
    rsx! {
        div { class: "ds-user-picture", {body} }
    }
}

/// A photo `px` across, cropped round: `div.ds-user-photo[data-size]` clipping an
/// `img.ds-user-photo-image` that covers it. Decorative: the name beside it is what a screen
/// reader reads.
pub(crate) fn photo(source: ImageSource, px: u16) -> Element {
    rsx! {
        div { class: "ds-user-photo", "data-size": "{px}", "aria-hidden": "true",
            img { class: "ds-user-photo-image", alt: "", src: source.0, draggable: "false" }
        }
    }
}
