//! `org.quire.AccountsSheet1`, as the shell exports it. The object only checks who is calling and
//! hands the call to the loop, which decides and answers.

use porter_core::sheet::SheetView;
use sill_model::accounts::sheet::SheetHandle;
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use zbus::fdo;
use zbus::message::Header;

use super::callers::{Caller, from_service};
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
        opener: Caller,
        answer: oneshot::Sender<fdo::Result<()>>,
    },
    Update {
        handle: SheetHandle,
        view: Box<SheetView>,
        from: Caller,
        answer: oneshot::Sender<fdo::Result<()>>,
    },
    Close {
        handle: SheetHandle,
        from: Caller,
        answer: oneshot::Sender<fdo::Result<()>>,
    },
}

/// The sheet object.
pub(super) struct SheetObject {
    pub calls: UnboundedSender<Call>,
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
        let opener = from_service(connection, &header).await?;
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
        let from = from_service(connection, &header).await?;
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
        let from = from_service(connection, &header).await?;
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
