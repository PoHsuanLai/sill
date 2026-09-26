//! The agent's loop (lane b fills it; a stub that returns at once until then).
//!
//! What it does once filled: stays passive (publishes `Registration::Passive`, touches no bus)
//! until `Policy(Own)`; then connects to `setup.bus`, serves [`super::dbus::Agent`] at
//! [`super::AGENT_PATH`], finds the session id (`SessionChoice`: logind's `session/auto` `Id`),
//! and registers; resolves the helper route once (`HelperChoice`, [`super::helper::helper_route`]);
//! turns every `Call`, command and helper line into a [`super::PolkitEvent`] through
//! [`super::reduce`], choosing the identity with [`super::pick_identity`] and naming it from
//! `/etc/passwd` (`crate::auth::user_name_in_passwd`); runs one helper per `StartHelper` (a child
//! with `kill_on_drop`, or the socket) and reads its lines with
//! [`super::helper::parse_helper_line`]; keeps the answer from `Answer` only until `SendAnswer`
//! writes it; answers the D-Bus call on `Complete`; runs a `Fake` request's helper in-process
//! (accepting [`super::FAKE_SECRET`]); unregisters on `Policy(Off)` and when every control is
//! gone; publishes [`super::polkit_of`].

use tokio::sync::{mpsc, watch};

use super::Setup;
use super::types::{Polkit, PolkitCommand};

pub(super) async fn run(
    tx: watch::Sender<Polkit>,
    commands: mpsc::UnboundedReceiver<PolkitCommand>,
    setup: Setup,
) {
    let _ = (tx, commands, setup);
}
