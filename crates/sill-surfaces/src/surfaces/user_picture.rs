//! The person's picture as quire's `LockUser` carries it (`UserPicture`, quire Q243): helpers
//! shared by the lock screen and the polkit prompt.

use ds::{LockUser, UserPicture};

/// The initial an initials face shows, if the picture is one.
pub fn initial_of(user: &LockUser) -> Option<char> {
    match &user.picture {
        UserPicture::Face(face) => Some(face.initial),
        _ => None,
    }
}
