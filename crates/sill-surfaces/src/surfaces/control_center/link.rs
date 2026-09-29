//! `sill control-center toggle`: the daemon asks a bar to toggle its control center. The
//! control center is a popup of a bar, not a surface of the daemon's, so the daemon cannot show
//! it itself; it writes an ask here (a root context of every bar), and the bar on that output
//! feeds its menu session a toggle of `BarMenu::ControlCenter`.
//!
//! An xdg popup's grab wants the serial of a recent input event, and an IPC request has none;
//! the compositor may refuse the grab (the popup then shows ungrabbed or not at all).

use shell_host::OutputId;
use tokio::sync::watch;

/// One toggle request: which output's bar, and a serial so two toggles in a row are two
/// changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ControlCenterAsk {
    /// `None` before the first ask.
    pub output: Option<OutputId>,
    pub serial: u64,
}

/// The daemon's half and every bar's receiver. Clone is cheap.
#[derive(Debug, Clone)]
pub struct ControlCenterLink {
    ask: watch::Sender<ControlCenterAsk>,
}

impl Default for ControlCenterLink {
    fn default() -> Self {
        ControlCenterLink::new()
    }
}

impl ControlCenterLink {
    /// No ask yet.
    pub fn new() -> ControlCenterLink {
        ControlCenterLink {
            ask: watch::Sender::new(ControlCenterAsk::default()),
        }
    }

    /// Ask the bar on `output` to toggle its control center.
    pub fn toggle(&self, output: OutputId) {
        self.ask.send_modify(|ask| {
            *ask = ControlCenterAsk {
                output: Some(output),
                serial: ask.serial + 1,
            };
        });
    }

    /// A receiver for a bar to follow; a bar acts on asks for its own output that arrive after
    /// it subscribed.
    pub fn subscribe(&self) -> watch::Receiver<ControlCenterAsk> {
        self.ask.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::{ControlCenterAsk, ControlCenterLink};
    use shell_host::OutputId;

    #[test]
    fn every_toggle_is_a_new_ask_for_its_output() {
        let link = ControlCenterLink::new();
        let mut rx = link.subscribe();
        assert_eq!(*rx.borrow_and_update(), ControlCenterAsk::default());
        link.toggle(OutputId(3));
        link.toggle(OutputId(3));
        assert!(rx.has_changed().unwrap_or(false));
        assert_eq!(
            *rx.borrow_and_update(),
            ControlCenterAsk {
                output: Some(OutputId(3)),
                serial: 2
            }
        );
    }
}
