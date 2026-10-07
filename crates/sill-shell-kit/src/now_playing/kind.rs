//! Where playback is (design/26-DETAILS.md 5.2.10).

use ds::motion::detail::stamp::EventStamp;
use ds::style::icon::Icon;

/// Where the player is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Playback {
    /// Paused or stopped: the button offers Play; the position holds.
    #[default]
    Paused,
    /// Playing: the button offers Pause; the position advances a second at a time.
    Playing,
    /// Asked to play, waiting on data (the service's stamp for this wait); the button already
    /// offers Pause.
    Buffering(EventStamp),
}

impl Playback {
    /// The glyph of the action the button offers next (Replace off-up: the next action, A5).
    pub fn next_action(self) -> Icon {
        match self {
            Playback::Paused => Icon::Play,
            Playback::Playing | Playback::Buffering(_) => Icon::Pause,
        }
    }

    /// The button's name: the action a press takes.
    pub fn label(self) -> &'static str {
        match self {
            Playback::Paused => "Play",
            Playback::Playing | Playback::Buffering(_) => "Pause",
        }
    }

    /// Whether the position runs on its own between reports.
    pub(crate) fn clock(self) -> PositionClock {
        match self {
            Playback::Playing => PositionClock::Running,
            Playback::Paused | Playback::Buffering(_) => PositionClock::Held,
        }
    }
}

/// Whether a position advances by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PositionClock {
    /// A second at a time from the last report.
    Running,
    /// Where the last report put it.
    Held,
}

#[cfg(test)]
mod tests {
    use super::Playback;
    use ds::motion::detail::stamp::EventStamp;
    use ds::style::icon::Icon;

    #[test]
    fn the_button_offers_the_next_action() {
        let cases = [
            (Playback::Paused, Icon::Play, "Play"),
            (Playback::Playing, Icon::Pause, "Pause"),
            (Playback::Buffering(EventStamp(1)), Icon::Pause, "Pause"),
        ];
        for (playback, icon, label) in cases {
            assert_eq!(
                (playback.next_action(), playback.label()),
                (icon, label),
                "{playback:?}"
            );
        }
    }
}
