//! The words a `NotificationCard` is described in: which app sent it, how many a
//! group holds and how many layers show behind it, the actions it offers, and whether the
//! pointer is over it.

use dioxus::prelude::*;
use ds::base::press::Press;
use ds::base::word::Word;
use ds::components::content::icon_source::IconSource;
use ds::components::content::text_runs::TextLine;

/// The app a notification came from: its icon (drawn at `--notifications-icon`, 32) and its
/// name (read to assistive technology before the summary).
#[derive(Debug, Clone, PartialEq)]
pub struct AppMark {
    /// The app's icon.
    pub icon: IconSource,
    /// The app's name.
    pub name: TextLine,
}

/// How many layers show behind a grouped card, each `--notifications-group-offset` (4) lower
/// than the one above it and a little narrower (design/13 section 13.3.6 draws two). Held to
/// [`StackLayers::MAX`]: more would read as a pile, not a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StackLayers(pub u8);

impl StackLayers {
    /// The most layers drawn.
    pub const MAX: u8 = 3;

    /// The layers drawn: 0 to [`Self::MAX`].
    pub fn drawn(self) -> u8 {
        self.0.min(Self::MAX)
    }
}

/// A group's count: the chip beside the age, and the layers behind the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupCount {
    /// How many notifications the group holds; the chip shows it from 2.
    pub count: u32,
    /// The layers behind the card.
    pub layers: StackLayers,
}

/// One action a notification offers, drawn as a Mini button in the row that opens on hover.
/// Its press stays at the button: it never also opens the notification.
#[derive(Debug, Clone, PartialEq)]
pub struct CardAction {
    /// The button's words.
    pub label: TextLine,
    /// What the press does (the server sends `ActionInvoked` with the action's key).
    pub on_press: EventHandler<Press>,
}

/// Whether the pointer is over a card: `data-hover`. Handed to the caller as it changes, so its
/// hold timer can pause while the card is read (design/13 section 13.3.6: hover pauses the
/// timer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub enum Hover {
    /// The pointer is elsewhere: the body is clamped to two lines, the close button and the
    /// actions are hidden.
    #[default]
    #[word(slug = "off")]
    Away,
    /// The pointer is over the card: the body opens to six lines, the close button and the
    /// actions show.
    #[word(slug = "on")]
    Over,
}
