//! Who caused what the panel draws next (design/26-DETAILS.md R5): the person's key or press
//! anywhere in the panel is a `Touch::Contact`, taken by the render that follows it; anything
//! else (an answer arriving, a load landing) is `Touch::Remote`.

use dioxus::prelude::*;
use ds::detail::{Handled, Touch};

/// The panel's last contact, until a render takes it.
#[derive(Clone, Copy)]
pub(crate) struct Contacts {
    last: CopyValue<Touch>,
    /// Bumped by each contact, so a render follows it and takes it: a contact left untaken
    /// would be spent on the next remote change.
    heard: Signal<u64>,
}

/// The panel's contacts.
pub(crate) fn use_contacts() -> Contacts {
    Contacts {
        last: use_hook(|| CopyValue::new(Touch::Remote)),
        heard: use_signal(|| 0),
    }
}

impl Contacts {
    /// A key or a press reached the panel's root (called from its handler).
    pub(crate) fn heard<T: Handled>(self, event: &Event<T>) {
        let (mut last, mut heard) = (self.last, self.heard);
        last.set(Touch::from_event(event));
        heard += 1;
    }

    /// The touch behind this render: the contact heard since the last render, else remote.
    pub(crate) fn take(self) -> Touch {
        let _ = (self.heard)();
        let mut last = self.last;
        let touch = *last.peek();
        if touch != Touch::Remote {
            last.set(Touch::Remote);
        }
        touch
    }
}
