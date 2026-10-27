//! The banner column's keymap: the keys a card takes once the column holds the keyboard, beyond
//! the ones quire's card takes itself (Enter and Space press the card or the button that has the
//! focus; Tab moves between the card's close button, its buttons and the next card by the
//! document's own order). Up and Down move along the stack as it is drawn, newest on top;
//! Escape, Delete and Backspace dismiss.

use dioxus::prelude::Key;

use super::deck_keys::{KeysIn, Neighbour};

/// What `key` does to the column; `None` leaves it to the document.
pub(super) fn banner_key(key: &Key) -> Option<KeysIn> {
    match key {
        // The newest card is the top one; older ones hang below it.
        Key::ArrowUp => Some(KeysIn::Moved(Neighbour::Newer)),
        Key::ArrowDown => Some(KeysIn::Moved(Neighbour::Older)),
        Key::Escape | Key::Delete | Key::Backspace => Some(KeysIn::Dismissed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::Key;

    use super::banner_key;
    use crate::banners::deck_keys::KeysIn::{Dismissed, Moved};
    use crate::banners::deck_keys::Neighbour::{Newer, Older};

    #[test]
    fn arrows_follow_the_stack_as_drawn_and_three_keys_dismiss() {
        let cases = [
            ("up", Key::ArrowUp, Some(Moved(Newer))),
            ("down", Key::ArrowDown, Some(Moved(Older))),
            ("Escape", Key::Escape, Some(Dismissed)),
            ("Delete", Key::Delete, Some(Dismissed)),
            ("Backspace", Key::Backspace, Some(Dismissed)),
            ("Enter is quire's", Key::Enter, None),
            ("Tab is the document's", Key::Tab, None),
            ("a letter", Key::Character("x".into()), None),
        ];
        for (name, key, want) in cases {
            assert_eq!(banner_key(&key), want, "{name}");
        }
    }
}
