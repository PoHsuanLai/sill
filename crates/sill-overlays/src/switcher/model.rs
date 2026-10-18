//! What the switcher keeps of its own around `ds_behaviour::switcher`, the machine the compositor
//! runs too: the direction sill's links carry.

use sill_ui_kit::links::Dir;

/// The machine's direction for the one the daemon's link carries.
pub fn heading(dir: Dir) -> ds_behaviour::Dir {
    match dir {
        Dir::Next => ds_behaviour::Dir::Next,
        Dir::Prev => ds_behaviour::Dir::Previous,
    }
}

#[cfg(test)]
mod tests {
    use sill_ui_kit::links::Dir;

    use super::heading;

    #[test]
    fn the_links_directions_map_onto_the_machines() {
        assert_eq!(heading(Dir::Next), ds_behaviour::Dir::Next);
        assert_eq!(heading(Dir::Prev), ds_behaviour::Dir::Previous);
    }
}
