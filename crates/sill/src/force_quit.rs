//! The daemon's half of the Force Quit Applications panel: the compositor's view of the desktop
//! fills the list on the link (`daemon::events`), the panel surfaces show it, and the app the
//! person confirmed comes back on the link. [`Outgoing`] sends each to the host as a command
//! for the desktop arm.

use sill_model::desktop::DesktopCommand;
use sill_ui_kit::links::{ForceQuitAsk, ForceQuitAsked, ForceQuitLink};
use tokio::sync::watch;

use crate::outgoing::Outgoing;

/// The command that ends the app `asked` names.
///
/// ```
/// use sill_model::desktop::DesktopCommand;
/// use sill_ui_kit::links::ForceQuitLink;
/// let link = ForceQuitLink::new();
/// link.ask("term");
/// let asked = link.last_asked().expect("asked");
/// assert_eq!(sill::force_quit::command_of(&asked), DesktopCommand::ForceQuitApp("term".to_owned()));
/// ```
pub fn command_of(asked: &ForceQuitAsked) -> DesktopCommand {
    DesktopCommand::ForceQuitApp(asked.app.clone())
}

impl Outgoing for ForceQuitLink {
    type Item = ForceQuitAsk;

    fn follow(&self) -> watch::Receiver<Option<ForceQuitAsked>> {
        self.subscribe_asked()
    }

    fn commands(asked: &ForceQuitAsked) -> Vec<DesktopCommand> {
        vec![command_of(asked)]
    }
}
