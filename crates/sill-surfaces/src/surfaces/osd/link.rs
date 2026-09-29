//! The last change the OSD was asked to show, shared by the daemon (which writes it) and every
//! output's OSD surface (which follows it), the way the launcher shares `LauncherLink`.
//!
//! A change names what moved, the outputs that show it, and where its level comes from: the
//! live services (volume, displays) for a real key press, or a fixed reading for `sill debug
//! osd`, which fakes a change without touching a real level. Each change carries one more
//! serial than the last, so a surface restarts its hold even when the same key is pressed again.

use shell_host::OutputId;
use sill_model::units::Percent;
use sill_model::volume::Mute;
use tokio::sync::watch;

/// What the indicator shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OsdKind {
    Volume,
    Brightness,
}

/// A level and mute, as the indicator draws them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Level {
    pub percent: Percent,
    pub mute: Mute,
}

/// Where a change's level is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LevelSource {
    /// The volume or displays service, as it publishes the new level.
    Live,
    /// A made-up reading (`sill debug osd`): nothing real moved.
    Fixed(Level),
}

/// Counts changes; each is one more than the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ChangeSerial(pub u64);

/// One change to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsdChange {
    pub serial: ChangeSerial,
    pub kind: OsdKind,
    /// The outputs whose indicator shows it.
    pub outputs: Vec<OutputId>,
    pub level: LevelSource,
}

impl OsdChange {
    /// Whether `output`'s indicator shows this change.
    pub fn shows_on(&self, output: OutputId) -> bool {
        self.outputs.contains(&output)
    }
}

/// The latest change, if any has been asked for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OsdLatest {
    #[default]
    None,
    Change(OsdChange),
}

impl OsdLatest {
    /// The latest change's serial (zero before the first).
    pub fn serial(&self) -> ChangeSerial {
        match self {
            OsdLatest::None => ChangeSerial::default(),
            OsdLatest::Change(change) => change.serial,
        }
    }

    /// The latest change, when it is newer than `seen` and shows on `output`.
    pub fn fresh_for(&self, seen: ChangeSerial, output: OutputId) -> Option<&OsdChange> {
        match self {
            OsdLatest::Change(change) if change.serial > seen && change.shows_on(output) => {
                Some(change)
            }
            _ => None,
        }
    }
}

/// `latest` followed by a change of `kind` on `outputs`, one serial on.
pub fn next(
    latest: &OsdLatest,
    kind: OsdKind,
    outputs: Vec<OutputId>,
    level: LevelSource,
) -> OsdLatest {
    OsdLatest::Change(OsdChange {
        serial: ChangeSerial(latest.serial().0 + 1),
        kind,
        outputs,
        level,
    })
}

/// The shared channel, a root context of every OSD surface and kept by the daemon. Clone is
/// cheap.
#[derive(Debug, Clone)]
pub struct OsdLink {
    latest: watch::Sender<OsdLatest>,
}

impl Default for OsdLink {
    fn default() -> Self {
        OsdLink::new()
    }
}

impl OsdLink {
    /// No change yet.
    pub fn new() -> OsdLink {
        OsdLink {
            latest: watch::Sender::new(OsdLatest::None),
        }
    }

    /// Show a change of `kind` on `outputs` (nothing when `outputs` is empty).
    pub fn show(&self, kind: OsdKind, outputs: Vec<OutputId>, level: LevelSource) {
        if outputs.is_empty() {
            return;
        }
        self.latest
            .send_modify(|latest| *latest = next(latest, kind, outputs, level));
    }

    /// The latest change.
    pub fn now(&self) -> OsdLatest {
        self.latest.borrow().clone()
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<OsdLatest> {
        self.latest.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::{ChangeSerial, LevelSource, OsdKind, OsdLatest, OsdLink, next};
    use shell_host::OutputId;

    const ONE: OutputId = OutputId(1);
    const TWO: OutputId = OutputId(2);

    #[test]
    fn each_change_counts_on_and_names_its_outputs() {
        let first = next(
            &OsdLatest::None,
            OsdKind::Volume,
            vec![ONE],
            LevelSource::Live,
        );
        assert_eq!(first.serial(), ChangeSerial(1));
        let second = next(&first, OsdKind::Volume, vec![ONE], LevelSource::Live);
        assert_eq!(second.serial(), ChangeSerial(2), "the same key again");
        const CASES: &[(u64, OutputId, bool)] = &[
            (0, ONE, true),
            (1, ONE, true),
            (2, ONE, false),
            (0, TWO, false),
        ];
        for &(seen, output, want) in CASES {
            assert_eq!(
                second.fresh_for(ChangeSerial(seen), output).is_some(),
                want,
                "seen {seen} on {output:?}"
            );
        }
        assert_eq!(OsdLatest::None.fresh_for(ChangeSerial(0), ONE), None);
    }

    #[test]
    fn a_change_for_no_output_is_not_sent() {
        let link = OsdLink::new();
        let mut rx = link.subscribe();
        link.show(OsdKind::Brightness, Vec::new(), LevelSource::Live);
        assert!(!rx.has_changed().unwrap_or(true));
        link.show(OsdKind::Brightness, vec![ONE, TWO], LevelSource::Live);
        assert!(rx.has_changed().unwrap_or(false));
        assert_eq!(rx.borrow_and_update().serial(), ChangeSerial(1));
    }
}
