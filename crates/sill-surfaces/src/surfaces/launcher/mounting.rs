//! When the palette is mounted anew. quire's palette plays its entrance, takes focus and puts
//! its selection on the first row only when it mounts, so a new mount is how an opening gets
//! them (and how the field gets the keyboard back from the actions menu). But unmounting a
//! palette, or a ds `Menu`, before its entrance has settled lets the entrance timer's task
//! outlive its signals and panic the whole process (FINDINGS Q45), so a palette is kept for at
//! least [`ENTRANCE_GUARD`]: an opening within it reuses the mounted palette, a refocus waits
//! for it. The palette stays mounted while the launcher is hidden. Pure.

use std::time::{Duration, Instant};

/// Longer than any entrance quire plays (`peek-in` over `--t-big` with its spring, `menu-pop`)
/// at any motion level.
pub const ENTRANCE_GUARD: Duration = Duration::from_millis(900);

/// The mounted palette: its key and when it mounted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PaletteMount {
    pub key: u64,
    pub at: Option<Instant>,
}

/// Whether a new mount may happen now or must wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remount {
    Now,
    After(Duration),
}

impl PaletteMount {
    /// Whether the palette may be replaced at `now`.
    pub fn remount_at(&self, now: Instant) -> Remount {
        match self.at {
            None => Remount::Now,
            Some(at) => match ENTRANCE_GUARD.checked_sub(now.saturating_duration_since(at)) {
                Some(left) if !left.is_zero() => Remount::After(left),
                _ => Remount::Now,
            },
        }
    }

    /// A new palette mounted at `now`.
    pub fn next(self, now: Instant) -> PaletteMount {
        PaletteMount {
            key: self.key + 1,
            at: Some(now),
        }
    }

    /// At an opening: a new palette when the guard allows, else this one.
    pub fn for_opening(self, now: Instant) -> PaletteMount {
        match self.remount_at(now) {
            Remount::Now => self.next(now),
            Remount::After(_) => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ENTRANCE_GUARD, PaletteMount, Remount};
    use std::time::{Duration, Instant};

    #[test]
    fn a_palette_is_kept_until_its_entrance_has_settled() {
        let t0 = Instant::now();
        let none = PaletteMount::default();
        assert_eq!(none.remount_at(t0), Remount::Now);
        let first = none.for_opening(t0);
        assert_eq!(
            first,
            PaletteMount {
                key: 1,
                at: Some(t0)
            }
        );
        let soon = t0 + Duration::from_millis(300);
        assert_eq!(
            first.for_opening(soon),
            first,
            "a quick reopening reuses it"
        );
        assert_eq!(
            first.remount_at(soon),
            Remount::After(ENTRANCE_GUARD - Duration::from_millis(300))
        );
        let later = t0 + ENTRANCE_GUARD;
        assert_eq!(first.for_opening(later).key, 2);
        assert_eq!(first.remount_at(later), Remount::Now);
    }
}
