//! What the switcher keeps of its own around `ds_behaviour::switcher`, the machine the compositor
//! runs too: the app key it is generic over, the direction sill's links carry, and the show
//! delay as the milliseconds a [`Stamp`] counts in.

use std::time::Duration;

use ds::base::time::stamp::Stamp;
use sill_launcher::AppId;
use sill_ui_kit::links::Dir;

/// What the machine asks of the surface, over sill's app ids.
pub type AppOut = ds_behaviour::switcher::SwOut<AppId>;

/// The machine's direction for the one the daemon's link carries.
pub fn heading(dir: Dir) -> ds_behaviour::Dir {
    match dir {
        Dir::Next => ds_behaviour::Dir::Next,
        Dir::Prev => ds_behaviour::Dir::Previous,
    }
}

/// The time `delay` after `at`.
pub fn after(at: Stamp, delay: Duration) -> Stamp {
    at.after(u64::try_from(delay.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ds::base::time::stamp::Stamp;
    use sill_ui_kit::links::Dir;

    use super::{after, heading};

    #[test]
    fn the_links_directions_map_onto_the_machines() {
        assert_eq!(heading(Dir::Next), ds_behaviour::Dir::Next);
        assert_eq!(heading(Dir::Prev), ds_behaviour::Dir::Previous);
    }

    #[test]
    fn a_delay_is_whole_milliseconds_after_the_stamp() {
        assert_eq!(after(Stamp(40), Duration::from_millis(150)), Stamp(190));
    }
}
