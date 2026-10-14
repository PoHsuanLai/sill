//! Whether the pointer has moved since the switcher came up. A tile that lands under a pointer
//! resting where the panel appeared is not chosen, since macOS follows the pointer only once it
//! moves; otherwise a held Cmd+Tab, Tab would be undone by whichever tile appeared under it. The
//! tile it rests on is kept, and chosen at the first move. Pure.

use sill_shell_kit::switcher::app_switcher::AppKey;

/// How far, in logical pixels, the pointer must go to count as moved.
const SLOP: f64 = 2.0;

/// A point on the surface, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct At {
    pub x: f64,
    pub y: f64,
}

/// What this opening has seen of the pointer.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Pointer {
    /// Nothing yet.
    #[default]
    Still,
    /// Not moved: where it was first seen, and the tile it rests on.
    Resting {
        at: Option<At>,
        over: Option<AppKey>,
    },
    Moving,
}

impl Pointer {
    /// The pointer seen at `at`: the state after, and the tile to choose now (the one it rested
    /// on, at its first move).
    pub fn moved(self, at: At) -> (Pointer, Option<AppKey>) {
        match self {
            Pointer::Still => (
                Pointer::Resting {
                    at: Some(at),
                    over: None,
                },
                None,
            ),
            Pointer::Resting { at: None, over } => (Pointer::Resting { at: Some(at), over }, None),
            Pointer::Resting {
                at: Some(first),
                over,
            } if (first.x - at.x).abs() + (first.y - at.y).abs() < SLOP => (
                Pointer::Resting {
                    at: Some(first),
                    over,
                },
                None,
            ),
            Pointer::Resting { over, .. } => (Pointer::Moving, over),
            Pointer::Moving => (Pointer::Moving, None),
        }
    }

    /// The pointer entered `key`'s tile: the state after, and the tile to choose now (none until
    /// the pointer has moved).
    pub fn entered(self, key: AppKey) -> (Pointer, Option<AppKey>) {
        match self {
            Pointer::Still => (
                Pointer::Resting {
                    at: None,
                    over: Some(key),
                },
                None,
            ),
            Pointer::Resting { at, .. } => (
                Pointer::Resting {
                    at,
                    over: Some(key),
                },
                None,
            ),
            Pointer::Moving => (Pointer::Moving, Some(key)),
        }
    }
}

#[cfg(test)]
mod tests {
    use sill_shell_kit::switcher::app_switcher::AppKey;

    use super::{At, Pointer};

    fn at(x: f64, y: f64) -> At {
        At { x, y }
    }

    fn key(name: &str) -> AppKey {
        AppKey(name.to_owned())
    }

    #[test]
    fn a_tile_under_a_resting_pointer_is_chosen_only_at_the_first_move() {
        let (p, chosen) = Pointer::default().moved(at(10.0, 10.0));
        assert_eq!(chosen, None);
        let (p, chosen) = p.entered(key("b"));
        assert_eq!(chosen, None);
        let (p, chosen) = p.moved(at(10.5, 10.5));
        assert_eq!(chosen, None, "inside the slop is still resting");
        let (p, chosen) = p.moved(at(14.0, 10.0));
        assert_eq!((p.clone(), chosen), (Pointer::Moving, Some(key("b"))));
        assert_eq!(p.entered(key("c")).1, Some(key("c")));
    }

    #[test]
    fn an_enter_before_any_move_waits_too() {
        let (p, chosen) = Pointer::default().entered(key("a"));
        assert_eq!(chosen, None);
        let (p, chosen) = p.moved(at(0.0, 0.0));
        assert_eq!(chosen, None);
        assert_eq!(p.moved(at(9.0, 0.0)).1, Some(key("a")));
    }
}
