//! The user's picture (design/25-EMOJI.md section 7): a letter disc, an animated emoji or their
//! own photo ([`UserPicture`]), drawn by the lock and polkit prompts; the
//! stored choice ([`PictureChoice`]) and the rule that resolves it ([`resolve_picture`]); and
//! [`UserPicturePicker`], where a person picks one.

pub mod choice;
pub(crate) mod draw;
pub(crate) mod picker;
pub mod picture;
pub mod size;
