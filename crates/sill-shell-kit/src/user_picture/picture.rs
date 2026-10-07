//! A user's picture, any kind (design/25-EMOJI.md section 7): the letter disc, an animated
//! emoji they picked, or their own photo (the freedesktop `~/.face` or AccountsService's icon).
//! A surface takes a [`UserPicture`] and draws it with [`super::UserPortrait`], so switching a
//! person from one kind to another changes data, not markup.

use crate::emoji::id::EmojiId;
use ds::components::content::avatar::AvatarFace;
use ds::components::content::image_source::ImageSource;

/// What stands for a user: a letter disc, an animated emoji or a photo.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UserPicture {
    /// A coloured disc with one letter, at its own size.
    Face(AvatarFace),
    /// An animated emoji from the shipped set, in the disc's size and place, playing the
    /// surface's mood (design/25-EMOJI.md section 5).
    Emoji(EmojiId),
    /// The user's own picture, cropped round to fill the disc (`object-fit: cover`), at the
    /// size a face would have there. Pass `$HOME/.face` or AccountsService's icon as
    /// `ImageSource::file(path)`.
    Photo(ImageSource),
}

impl From<AvatarFace> for UserPicture {
    fn from(face: AvatarFace) -> Self {
        UserPicture::Face(face)
    }
}

impl From<EmojiId> for UserPicture {
    fn from(emoji: EmojiId) -> Self {
        UserPicture::Emoji(emoji)
    }
}

impl From<ImageSource> for UserPicture {
    fn from(source: ImageSource) -> Self {
        UserPicture::Photo(source)
    }
}
