//! Who may drive the sheet: the owners of the three account services' names. accountd holds
//! the identities, inferd the AI keys and permits, syncd its own sign-in; each asks for the same
//! sheets with the same words, so each is checked against its own name the same way.

use porter_dbus::ACCOUNTS_BUS;
use zbus::Connection;
use zbus::fdo;
use zbus::message::Header;
use zbus::names::{BusName, OwnedUniqueName};

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
}

/// A caller that is the current owner of one of the services' names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Caller {
    /// Its unique name on the bus.
    pub name: OwnedUniqueName,
    /// The service it is the owner of.
    pub host: Host,
}

/// The caller of `header`, when it is the owner of one of the services' names; anyone else is
/// refused (`AccessDenied`).
pub(super) async fn from_service(
    connection: &Connection,
    header: &Header<'_>,
) -> fdo::Result<Caller> {
    let denied = || {
        fdo::Error::AccessDenied("only the account services drive the accounts sheet".to_owned())
    };
    let sender = header.sender().ok_or_else(denied)?;
    let bus = fdo::DBusProxy::new(connection).await?;
    for host in Host::ALL {
        let name = BusName::try_from(host.bus()).map_err(|e| fdo::Error::Failed(e.to_string()))?;
        // A name nobody owns has no owner to match.
        if let Ok(owner) = bus.get_name_owner(name).await
            && sender.as_str() == owner.as_str()
        {
            return Ok(Caller { name: owner, host });
        }
    }
    Err(denied())
}
