//! The user's stored choice of picture (a settings value, proposed `session.user_picture`), and
//! the pure rule that turns it into a [`UserPicture`] to draw (design/25-EMOJI.md section 7).

use super::picture::UserPicture;
use crate::emoji::id::EmojiId;
use ds::components::content::avatar::AvatarFace;
use ds::components::content::image_source::ImageSource;
use serde::{Deserialize, Serialize};

/// Which kind of picture the user chose. Stored adjacently tagged (`{"kind":"emoji","v":
/// "heart-eyes"}`); an emoji is stored by its stable slug, so reordering the set keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum PictureChoice {
    /// Nothing chosen: their photo when `~/.face` exists, else the letter.
    #[default]
    Auto,
    /// The letter disc, even when a photo exists.
    Letter,
    /// This animated emoji.
    Emoji(EmojiId),
    /// Their photo; the letter while there is none.
    Photo,
}

/// Whether the user's photo (`$HOME/.face`, or AccountsService's icon) was found, and where.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FaceFile {
    /// Found: this picture.
    Found(ImageSource),
    /// There is none.
    Missing,
}

/// What to draw for `choice`, given whether a photo was found (`face`) and the letter disc the
/// user's name gives (`letter`). A choice that needs a photo falls back to the letter when there
/// is none, so a picture is always drawn.
pub fn resolve_picture(choice: PictureChoice, face: FaceFile, letter: AvatarFace) -> UserPicture {
    match (choice, face) {
        (PictureChoice::Emoji(emoji), _) => UserPicture::Emoji(emoji),
        (PictureChoice::Letter, _) => UserPicture::Face(letter),
        (PictureChoice::Auto | PictureChoice::Photo, FaceFile::Found(source)) => {
            UserPicture::Photo(source)
        }
        (PictureChoice::Auto | PictureChoice::Photo, FaceFile::Missing) => {
            UserPicture::Face(letter)
        }
    }
}
