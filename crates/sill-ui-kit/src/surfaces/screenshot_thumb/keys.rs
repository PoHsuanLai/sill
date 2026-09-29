//! The thumbnail's keymap: the keys the card takes once it holds the keyboard, beyond the ones
//! quire's picture takes itself (Enter and Space open it; Tab moves between the picture and the
//! card's action buttons by the document's own order).

use dioxus::prelude::Key;

use super::step::ThumbIn;

/// What `key` does to the card; `None` leaves it to the document.
pub(super) fn thumb_key(key: &Key) -> Option<ThumbIn> {
    match key {
        Key::Delete | Key::Backspace => Some(ThumbIn::TrashPressed),
        Key::Escape => Some(ThumbIn::Dismissed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::Key;

    use super::thumb_key;
    use crate::surfaces::screenshot_thumb::step::ThumbIn;

    #[test]
    fn delete_and_backspace_trash_it_and_escape_dismisses_it() {
        let cases = [
            ("Delete", Key::Delete, Some(ThumbIn::TrashPressed)),
            ("Backspace", Key::Backspace, Some(ThumbIn::TrashPressed)),
            ("Escape", Key::Escape, Some(ThumbIn::Dismissed)),
            ("Enter is quire's", Key::Enter, None),
            ("Tab is the document's", Key::Tab, None),
            ("a letter", Key::Character("x".into()), None),
        ];
        for (name, key, want) in cases {
            assert_eq!(thumb_key(&key), want, "{name}");
        }
    }
}
