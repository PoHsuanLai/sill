//! `org.quire.Displays1`, as the shell exports it. Reading is open; setting checks the caller
//! is Settings, then hands the change to the displays service.

use std::sync::Arc;

use peephole_bus::peephole_core::roles;
use sill_model::displays::{Displays, DisplaysCommand, wire_of};
use sill_model::ids::OutputName;
use sill_model::units::Percent;
use tokio::sync::watch;
use zbus::fdo;
use zbus::interface;
use zbus::message::Header;
use zbus::object_server::SignalEmitter;

use crate::callers::{Claim, Peers, admit};
use crate::control::Control;

/// Settings' name: only its owner, holding the `settings` role, may set a brightness.
const SETTINGS_BUS: &str = "org.quire.Settings";

/// The displays object.
pub(super) struct DisplaysObject {
    pub displays: watch::Receiver<Displays>,
    pub control: Control<DisplaysCommand>,
    /// Tells the callers on the session connection apart.
    pub peers: Arc<Peers>,
}

#[interface(name = "org.quire.Displays1")]
impl DisplaysObject {
    /// The outputs whose brightness can be read, with their percentage.
    fn brightness(&self) -> Vec<(String, u32)> {
        wire_of(&self.displays.borrow().levels())
    }

    async fn set_brightness(
        &self,
        output: String,
        percent: u32,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<()> {
        let claims = [Claim {
            name: SETTINGS_BUS,
            role: roles::SETTINGS,
            tag: (),
        }];
        admit(&self.peers, &header, &claims).await?;
        let level = match u16::try_from(percent) {
            Ok(level) if level <= 100 => Percent(level),
            _ => return Err(fdo::Error::InvalidArgs("percent is 0 to 100".to_owned())),
        };
        let output = OutputName(output);
        if !self
            .displays
            .borrow()
            .levels()
            .iter()
            .any(|l| l.output == output)
        {
            return Err(fdo::Error::InvalidArgs("no such display".to_owned()));
        }
        self.control
            .send(DisplaysCommand::SetBrightness { output, level })
            .map_err(|_| fdo::Error::Failed("the displays have stopped".to_owned()))
    }

    /// The list again, when any level changed.
    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>, levels: Vec<(String, u32)>) -> zbus::Result<()>;
}
