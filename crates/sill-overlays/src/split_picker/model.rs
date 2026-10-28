//! What the picker decides, pure: which output shows an offer, which half of the screen the tiles
//! fill, and where the selection goes.

use ds_behaviour::Dir;
use sill_model::desktop::SplitSide;

/// Whether the offer made for the output called `offer_output` (empty when the desktop does not
/// say) shows on the output named `name`.
pub(super) fn shows_on(offer_output: &str, name: Option<&str>) -> bool {
    offer_output.is_empty() || name == Some(offer_output)
}

/// The half of the screen the tiles fill: the one the first window did not take.
pub(super) fn other_half(taken: SplitSide) -> &'static str {
    match taken {
        SplitSide::Left => "right",
        SplitSide::Right => "left",
    }
}

/// The index one step in `dir` from `at` among `len` tiles, wrapping round at either end; 0 for no
/// tiles.
pub(super) fn moved(len: usize, at: usize, dir: Dir) -> usize {
    match (len, dir) {
        (0, _) => 0,
        (len, Dir::Next) => (at + 1) % len,
        (len, Dir::Previous) => (at + len - 1) % len,
    }
}

/// The index of the window `key` among the tiles' windows.
pub(super) fn index_of(windows: &[&str], key: &str) -> Option<usize> {
    windows.iter().position(|window| *window == key)
}

#[cfg(test)]
mod tests {
    use ds_behaviour::Dir;
    use sill_model::desktop::SplitSide;

    use super::{index_of, moved, other_half, shows_on};

    /// (case, the offer's output, this output's name, shows).
    type ShowCase = (&'static str, &'static str, Option<&'static str>, bool);

    #[test]
    fn an_offer_shows_on_its_output_or_on_every_one_when_it_names_none() {
        let cases: [ShowCase; 4] = [
            ("its own output", "DP-1", Some("DP-1"), true),
            ("another output", "DP-1", Some("DP-2"), false),
            ("an output with no name", "DP-1", None, false),
            ("no output named", "", Some("DP-2"), true),
        ];
        for (name, offered, here, want) in cases {
            assert_eq!(shows_on(offered, here), want, "{name}");
        }
    }

    #[test]
    fn the_tiles_fill_the_half_the_first_window_left_free() {
        assert_eq!(other_half(SplitSide::Left), "right");
        assert_eq!(other_half(SplitSide::Right), "left");
    }

    /// (case, tiles, at, step, lands on).
    type MoveCase = (&'static str, usize, usize, Dir, usize);

    #[test]
    fn the_selection_moves_and_wraps() {
        let cases: [MoveCase; 6] = [
            ("forward", 3, 0, Dir::Next, 1),
            ("forward off the end wraps", 3, 2, Dir::Next, 0),
            ("back", 3, 2, Dir::Previous, 1),
            ("back off the start wraps", 3, 0, Dir::Previous, 2),
            ("one tile stays", 1, 0, Dir::Next, 0),
            ("no tiles", 0, 0, Dir::Previous, 0),
        ];
        for (name, len, at, step, want) in cases {
            assert_eq!(moved(len, at, step), want, "{name}");
        }
    }

    #[test]
    fn a_tile_is_found_by_its_window() {
        let windows = ["w2", "w5"];
        assert_eq!(index_of(&windows, "w5"), Some(1));
        assert_eq!(index_of(&windows, "w9"), None);
    }
}
