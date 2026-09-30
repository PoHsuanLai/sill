//! The banner column's keymap: the keys a card takes once the column holds the keyboard, beyond
//! the ones quire's card takes itself (Enter and Space press the card or the button that has the
//! focus; Tab moves between the card's close button, its buttons and the next card by the
//! document's own order). Up and Down move along the stack as it is drawn, so they follow the
//! corner the stack stands in; Escape, Delete and Backspace dismiss.

use dioxus::prelude::Key;
use sill_settings::BannerPosition;

use super::deck_keys::{KeysIn, Neighbour};

/// What `key` does to the column with its stack in `corner`; `None` leaves it to the document.
pub(super) fn banner_key(key: &Key, corner: BannerPosition) -> Option<KeysIn> {
    let (up, down) = match corner {
        // The newest card is the top one; older ones hang below it.
        BannerPosition::TopRight => (Neighbour::Newer, Neighbour::Older),
        // The newest card is the bottom one; older ones stand above it.
        BannerPosition::BottomRight => (Neighbour::Older, Neighbour::Newer),
    };
    match key {
        Key::ArrowUp => Some(KeysIn::Moved(up)),
        Key::ArrowDown => Some(KeysIn::Moved(down)),
        Key::Escape | Key::Delete | Key::Backspace => Some(KeysIn::Dismissed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use dioxus::prelude::Key;
    use sill_settings::BannerPosition::{BottomRight, TopRight};

    use super::banner_key;
    use crate::surfaces::banners::deck_keys::KeysIn::{Dismissed, Moved};
    use crate::surfaces::banners::deck_keys::Neighbour::{Newer, Older};

    #[test]
    fn arrows_follow_the_stack_as_drawn_and_three_keys_dismiss() {
        let cases = [
            ("up at the top", Key::ArrowUp, TopRight, Some(Moved(Newer))),
            (
                "down at the top",
                Key::ArrowDown,
                TopRight,
                Some(Moved(Older)),
            ),
            (
                "up at the bottom",
                Key::ArrowUp,
                BottomRight,
                Some(Moved(Older)),
            ),
            (
                "down at the bottom",
                Key::ArrowDown,
                BottomRight,
                Some(Moved(Newer)),
            ),
            ("Escape", Key::Escape, TopRight, Some(Dismissed)),
            ("Delete", Key::Delete, BottomRight, Some(Dismissed)),
            ("Backspace", Key::Backspace, TopRight, Some(Dismissed)),
            ("Enter is quire's", Key::Enter, TopRight, None),
            ("Tab is the document's", Key::Tab, TopRight, None),
            ("a letter", Key::Character("x".into()), TopRight, None),
        ];
        for (name, key, corner, want) in cases {
            assert_eq!(banner_key(&key, corner), want, "{name}");
        }
    }
}
