//! Where the sheets open. The questions that ask the person something (polkit, confirm, voice
//! consent, accounts, capture, permission, the display trial) open on the focused window's output,
//! else the first output (the daemon does not know the pointer's), like a panel
//! (`Outputs::open_on`). They must not follow the focus afterwards: a sheet takes clicks on its
//! card only, so a click beside it changes the focus, and the question would jump outputs.
//!
//! One latch on the host thread decides it for all seven. [`spawn_follower`] nudges the host
//! (`ShellMsg::Sheets`) whenever a source of questions changes; [`SheetLatch::next`] takes the
//! focused output when the first question of a burst appears, holds it while any question is up
//! and lets go when none is; [`SheetLatch::output`] turns it into the output the surfaces
//! follow (`SheetLink`), the first one when the latched output was unplugged. Until the latch
//! takes hold no output shows a sheet, so a question never flashes on the wrong screen.

use shell_host::HostHandle;
use sill_model::ids::OutputName;
use sill_model::intents::sheet::sheet_view;
use sill_model::voice::ConsentSheet;
use sill_ui_kit::links::{Links, SheetOutput};
use sill_ui_kit::services::Services;
use sill_ui_kit::surface::OutputPolicy;
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::daemon::msg::ShellMsg;
use crate::outputs::Outputs;

/// Whether any question is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asking {
    Nothing,
    Something,
}

impl Asking {
    /// `Something` when `question` is there.
    fn of<T>(question: Option<T>) -> Asking {
        match question {
            Some(_) => Asking::Something,
            None => Asking::Nothing,
        }
    }

    /// `Something` when either is.
    fn or(self, other: Asking) -> Asking {
        match self {
            Asking::Something => Asking::Something,
            Asking::Nothing => other,
        }
    }

    /// Whether any question of the daemon's sources is up now.
    pub fn now(services: &Services, links: &Links) -> Asking {
        let capture = services.capture.state.borrow();
        let consent = match services.voice.state.borrow().sheet {
            ConsentSheet::Asking => Asking::Something,
            ConsentSheet::Closed => Asking::Nothing,
        };
        let intents = Asking::of(sheet_view(&services.intents.state.borrow().sheets));
        let polkit = Asking::of(services.polkit_agent.state.borrow().current.as_ref());
        let accounts = Asking::of(services.accounts_sheet.state.borrow().current());
        let trial = Asking::of(links.display_trial.latest());
        intents
            .or(polkit)
            .or(Asking::of(capture.asking.as_ref()))
            .or(Asking::of(capture.permission.as_ref()))
            .or(consent)
            .or(accounts)
            .or(trial)
    }
}

/// The output a burst of questions was started on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SheetLatch {
    /// No question is up.
    #[default]
    Idle,
    /// Questions are up, since the focused window was on this output (`None`: no focused window).
    Held(Option<OutputName>),
}

impl SheetLatch {
    /// The latch after the sources changed: `focused` is the focused window's output now, taken
    /// only when a burst starts.
    pub fn next(self, asking: Asking, focused: Option<OutputName>) -> SheetLatch {
        match (self, asking) {
            (_, Asking::Nothing) => SheetLatch::Idle,
            (SheetLatch::Idle, Asking::Something) => SheetLatch::Held(focused),
            (held @ SheetLatch::Held(_), Asking::Something) => held,
        }
    }

    /// The output the sheets show on: none while idle, else the latched output, or the first one
    /// when that is not there (unplugged, or never known).
    pub fn output(&self, outputs: &Outputs) -> SheetOutput {
        match self {
            SheetLatch::Idle => SheetOutput::Nowhere,
            SheetLatch::Held(focused) => outputs
                .pick(OutputPolicy::FocusedWindow, focused.as_ref())
                .map_or(SheetOutput::Nowhere, SheetOutput::On),
        }
    }

    /// The latch after a nudge, with its output written to the link.
    pub fn follow(self, services: &Services, links: &Links, outputs: &Outputs) -> SheetLatch {
        let focused = services.windows.state.borrow().toplevels.focused_output();
        let next = self.next(Asking::now(services, links), focused);
        links.sheets.set(next.output(outputs));
        next
    }
}

/// Tell the host to look at the questions again whenever one of the sources changes, until the
/// host is gone.
pub fn spawn_follower(
    rt: &Handle,
    services: &Services,
    links: &Links,
    host: HostHandle<ShellMsg>,
) {
    nudge(rt, services.intents.state.clone(), &host);
    nudge(rt, services.polkit_agent.state.clone(), &host);
    nudge(rt, services.capture.state.clone(), &host);
    nudge(rt, services.voice.state.clone(), &host);
    nudge(rt, services.accounts_sheet.state.clone(), &host);
    nudge(rt, links.display_trial.subscribe(), &host);
}

/// Send `ShellMsg::Sheets` each time `source` changes.
fn nudge<T: Send + Sync + 'static>(
    rt: &Handle,
    mut source: watch::Receiver<T>,
    host: &HostHandle<ShellMsg>,
) {
    let host = host.clone();
    rt.spawn(async move {
        while source.changed().await.is_ok() {
            if host.send(ShellMsg::Sheets).is_err() {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use sill_model::ids::OutputName;

    use super::{Asking, SheetLatch};
    use crate::outputs::Outputs;
    use sill_ui_kit::links::SheetOutput;

    fn on(name: &str) -> Option<OutputName> {
        Some(OutputName(name.to_owned()))
    }

    #[test]
    fn a_burst_takes_the_focus_once_and_keeps_it_until_no_question_is_left() {
        use Asking::{Nothing, Something};
        use SheetLatch::{Held, Idle};
        let (a, b) = (on("A"), on("B"));
        let held = |at: &Option<OutputName>| Held(at.clone());
        let cases = [
            ("idle stays idle", Idle, Nothing, a.clone(), Idle),
            ("the first takes the focus", Idle, Something, a.clone(), held(&a)),
            ("no focused window is held", Idle, Something, None, Held(None)),
            ("a click elsewhere keeps it", held(&a), Something, b.clone(), held(&a)),
            ("focus lost keeps it", held(&a), Something, None, held(&a)),
            ("the last question lets go", held(&a), Nothing, a.clone(), Idle),
            ("the next burst takes the new focus", Idle, Something, b.clone(), held(&b)),
        ];
        for (name, before, asking, focused, after) in cases {
            assert_eq!(before.next(asking, focused), after, "{name}");
        }
    }

    #[test]
    fn no_output_shows_while_idle_or_without_outputs() {
        let outputs = Outputs::default();
        assert_eq!(SheetLatch::Idle.output(&outputs), SheetOutput::Nowhere);
        assert_eq!(
            SheetLatch::Held(on("A")).output(&outputs),
            SheetOutput::Nowhere
        );
    }
}
