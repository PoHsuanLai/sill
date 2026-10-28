//! The last change the OSD was asked to show, shared by the daemon (which writes it) and every
//! output's OSD surface (which follows it), the way the launcher shares `LauncherLink`.
//!
//! A change names what moved, the outputs that show it, and where its level comes from: the
//! live services (volume, displays) for a real key press, or a fixed reading for `sill debug
//! osd`, which fakes a change without touching a real level. Each change carries one more
//! serial than the last, so a surface restarts its hold even when the same key is pressed again.

use shell_host::OutputId;
pub use sill_model::osd::OsdKind;
use sill_model::units::Percent;
use sill_model::volume::Mute;
use tokio::sync::watch;

use super::Shared;

/// A level and mute, as the indicator draws them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Level {
    pub percent: Percent,
    pub mute: Mute,
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
    /// A made-up reading (`sill debug osd`: nothing real moved), or `None` for the level the
    /// volume or displays service publishes now.
    pub level: Option<Level>,
}

impl OsdChange {
    /// Whether `output`'s indicator shows this change.
    pub fn shows_on(&self, output: OutputId) -> bool {
        self.outputs.contains(&output)
    }

    /// Whether `output`'s indicator has this change to show: it is newer than `seen`.
    pub fn is_fresh_for(&self, seen: ChangeSerial, output: OutputId) -> bool {
        self.serial > seen && self.shows_on(output)
    }
}

/// `latest` followed by a change of `kind` on `outputs`, one serial on.
pub fn next(
    latest: Option<&OsdChange>,
    kind: OsdKind,
    outputs: Vec<OutputId>,
    level: Option<Level>,
) -> OsdChange {
    OsdChange {
        serial: ChangeSerial(latest.map_or(0, |change| change.serial.0) + 1),
        kind,
        outputs,
        level,
    }
}

/// The shared channel, a root context of every OSD surface and kept by the daemon. Clone is
/// cheap.
#[derive(Debug, Clone, Default)]
pub struct OsdLink {
    latest: Shared<Option<OsdChange>>,
}

impl OsdLink {
    /// No change yet.
    pub fn new() -> OsdLink {
        OsdLink::default()
    }

    /// Show a change of `kind` on `outputs` (nothing when `outputs` is empty).
    pub fn show(&self, kind: OsdKind, outputs: Vec<OutputId>, level: Option<Level>) {
        if outputs.is_empty() {
            return;
        }
        self.latest
            .modify(|latest| *latest = Some(next(latest.as_ref(), kind, outputs, level)));
    }

    /// The latest change, if any has been asked for.
    pub fn now(&self) -> Option<OsdChange> {
        self.latest.now()
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<Option<OsdChange>> {
        self.latest.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::next;
    use crate::links::{ChangeSerial, OsdKind, OsdLink};
    use shell_host::OutputId;

    const ONE: OutputId = OutputId(1);
    const TWO: OutputId = OutputId(2);

    #[test]
    fn each_change_counts_on_and_names_its_outputs() {
        let first = next(None, OsdKind::Volume, vec![ONE], None);
        assert_eq!(first.serial, ChangeSerial(1));
        let second = next(Some(&first), OsdKind::Volume, vec![ONE], None);
        assert_eq!(second.serial, ChangeSerial(2), "the same key again");
        const CASES: &[(u64, OutputId, bool)] = &[
            (0, ONE, true),
            (1, ONE, true),
            (2, ONE, false),
            (0, TWO, false),
        ];
        for &(seen, output, want) in CASES {
            assert_eq!(
                second.is_fresh_for(ChangeSerial(seen), output),
                want,
                "seen {seen} on {output:?}"
            );
        }
    }

    #[test]
    fn a_change_for_no_output_is_not_sent() {
        let link = OsdLink::new();
        let mut rx = link.subscribe();
        link.show(OsdKind::Brightness, Vec::new(), None);
        assert!(!rx.has_changed().unwrap_or(true));
        link.show(OsdKind::Brightness, vec![ONE, TWO], None);
        assert!(rx.has_changed().unwrap_or(false));
        assert_eq!(
            rx.borrow_and_update().as_ref().map(|change| change.serial),
            Some(ChangeSerial(1))
        );
    }
}
