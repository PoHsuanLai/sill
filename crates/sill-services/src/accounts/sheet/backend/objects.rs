//! `org.quire.AccountsSheet1`, as the shell exports it. The object only checks who is calling and
//! hands the call to the loop, which decides and answers.

use porter_core::sheet::SheetView;
use porter_dbus::ACCOUNTS_BUS;
use sill_model::accounts::sheet::SheetHandle;
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use zbus::fdo;
use zbus::message::Header;
use zbus::names::{BusName, OwnedUniqueName};
use zbus::object_server::SignalEmitter;
use zbus::{Connection, interface};

/// The largest view taken, in bytes: a provider list or a review is a few kilobytes.
const MAX_VIEW: usize = 64 * 1024;

/// What the loop is asked.
#[derive(Debug)]
pub(super) enum Call {
    Open {
        handle: SheetHandle,
        view: Box<SheetView>,
        opener: OwnedUniqueName,
        answer: oneshot::Sender<fdo::Result<()>>,
    },
    Update {
        handle: SheetHandle,
        view: Box<SheetView>,
        from: OwnedUniqueName,
        answer: oneshot::Sender<fdo::Result<()>>,
    },
    Close {
        handle: SheetHandle,
        from: OwnedUniqueName,
        answer: oneshot::Sender<fdo::Result<()>>,
    },
}

/// The sheet object.
pub(super) struct SheetObject {
    pub calls: UnboundedSender<Call>,
}

/// The caller, when it is the owner of accountd's name: nobody else may open, change or close a
/// sheet in accountd's name.
async fn from_accountd(
    connection: &Connection,
    header: &Header<'_>,
) -> fdo::Result<OwnedUniqueName> {
    let denied = || fdo::Error::AccessDenied("only accountd drives the accounts sheet".to_owned());
    let name = BusName::try_from(ACCOUNTS_BUS).map_err(|e| fdo::Error::Failed(e.to_string()))?;
    let owner = fdo::DBusProxy::new(connection)
        .await?
        .get_name_owner(name)
        .await
        .map_err(|_| denied())?;
    match header.sender() {
        Some(sender) if sender.as_str() == owner.as_str() => Ok(owner),
        Some(_) | None => Err(denied()),
    }
}

fn handle_of(text: &str) -> fdo::Result<SheetHandle> {
    SheetHandle::parse(text).ok_or_else(|| fdo::Error::InvalidArgs("not a sheet handle".to_owned()))
}

fn view_of(json: &str) -> fdo::Result<Box<SheetView>> {
    if json.len() > MAX_VIEW {
        return Err(fdo::Error::InvalidArgs("the view is too large".to_owned()));
    }
    serde_json::from_str(json)
        .map(Box::new)
        .map_err(|e| fdo::Error::InvalidArgs(format!("not a SheetView: {e}")))
}

async fn asked(
    calls: &UnboundedSender<Call>,
    call: impl FnOnce(oneshot::Sender<fdo::Result<()>>) -> Call,
) -> fdo::Result<()> {
    let (answer, answered) = oneshot::channel();
    calls
        .send(call(answer))
        .map_err(|_| fdo::Error::Failed("the sheet has stopped".to_owned()))?;
    answered
        .await
        .map_err(|_| fdo::Error::Failed("the sheet has stopped".to_owned()))?
}

#[interface(name = "org.quire.AccountsSheet1")]
impl SheetObject {
    async fn open(
        &self,
        handle: String,
        _parent_window: String,
        view: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<()> {
        let opener = from_accountd(connection, &header).await?;
        let (handle, view) = (handle_of(&handle)?, view_of(&view)?);
        asked(&self.calls, |answer| Call::Open {
            handle,
            view,
            opener,
            answer,
        })
        .await
    }

    async fn update(
        &self,
        handle: String,
        view: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<()> {
        let from = from_accountd(connection, &header).await?;
        let (handle, view) = (handle_of(&handle)?, view_of(&view)?);
        asked(&self.calls, |answer| Call::Update {
            handle,
            view,
            from,
            answer,
        })
        .await
    }

    async fn close(
        &self,
        handle: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<()> {
        let from = from_accountd(connection, &header).await?;
        let handle = handle_of(&handle)?;
        asked(&self.calls, |answer| Call::Close {
            handle,
            from,
            answer,
        })
        .await
    }

    /// What the person did; sent to accountd alone (the loop emits it with a destination).
    #[zbus(signal)]
    async fn input(emitter: &SignalEmitter<'_>, handle: &str, input: &str) -> zbus::Result<()>;
}
