//! Who may drive the sheet: the account services. accountd holds the identities, inferd the AI
//! keys and permits, syncd its own sign-in; each asks for the same sheets with the same words.
//! A caller is let in when the system shows it to be the service whose well-known name it owns
//! (`crate::callers`): a program that merely claimed an unowned name is a stranger.

use peephole_bus::peephole_core::roles;
use porter_dbus::names::ACCOUNTS_BUS;
use zbus::fdo;
use zbus::message::Header;
use zbus::names::OwnedUniqueName;

use crate::callers::{Claim, Peers, admit};

/// A service that may ask for a sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Host {
    /// accountd: identities.
    Accounts,
    /// inferd: AI keys, the AI permit sheet and the agent key sheet.
    Inference,
    /// syncd: its own sign-in.
    Sync,
}

impl Host {
    /// Every service that may ask.
    pub(crate) const ALL: [Host; 3] = [Host::Accounts, Host::Inference, Host::Sync];

    /// The service's well-known bus name.
    pub(crate) const fn bus(self) -> &'static str {
        match self {
            Host::Accounts => ACCOUNTS_BUS,
            Host::Inference => "org.quire.Inference1",
            Host::Sync => "org.quire.Sync1",
        }
    }

    /// The claim the service makes: its name, and the role the desktop's table gives its program.
    fn claim(self) -> Claim<Host> {
        let role = match self {
            Host::Accounts => roles::ACCOUNTS,
            Host::Inference => roles::INFERENCE,
            Host::Sync => roles::SYNC,
        };
        Claim {
            name: self.bus(),
            role,
            tag: self,
        }
    }
}

/// A caller that is one of the services, by the unique name it was let in under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Caller {
    /// Its unique name on the bus: the only address an `Input` signal is sent to.
    pub name: OwnedUniqueName,
    /// The service it is.
    pub host: Host,
}

/// The caller of `header`, when it is one of the account services; anyone else is refused
/// (`AccessDenied`).
pub(super) async fn from_service(peers: &Peers, header: &Header<'_>) -> fdo::Result<Caller> {
    let claims = Host::ALL.map(Host::claim);
    let (name, host) = admit(peers, header, &claims).await?;
    Ok(Caller { name, host })
}
