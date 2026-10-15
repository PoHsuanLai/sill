//! `org.quire.Shell1`, as the shell exports it on the seat. The object checks who is calling and
//! hands the call to the loop, which decides, and waits for the loop's answer.

use sill_model::capture_pick::{Ask, ColorAsk, ColorReply, Reply};
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use zbus::fdo;
use zbus::message::Header;
use zbus::names::BusName;
use zbus::{Connection, interface};

/// The portal backend's name: transom owns it, and only its owner may ask the person.
pub(super) const PORTAL_BUS: &str = "org.freedesktop.impl.portal.desktop.quire";
/// Where the object is exported.
pub(super) const PATH: &str = "/org/quire/Shell1";

/// The longest app label taken, in characters.
const MAX_LABEL: usize = 128;
/// The most cursor modes taken.
const MAX_MODES: usize = 8;

/// What the loop is asked.
#[derive(Debug)]
pub(super) enum Call {
    /// What to share.
    Sources(SourceCall),
    /// One colour from the screen.
    Color(ColorCall),
}

/// A request for sources to share, and where its answer goes.
#[derive(Debug)]
pub(super) struct SourceCall {
    pub ask: Ask,
    pub answer: oneshot::Sender<fdo::Result<Reply>>,
}

/// A request for one colour, and where its answer goes.
#[derive(Debug)]
pub(super) struct ColorCall {
    pub ask: ColorAsk,
    pub answer: oneshot::Sender<fdo::Result<ColorReply>>,
}

/// The picker object.
pub(super) struct PickerObject {
    pub calls: UnboundedSender<Call>,
}

async fn from_transom(connection: &Connection, header: &Header<'_>) -> fdo::Result<()> {
    let denied = || fdo::Error::AccessDenied("only the portal backend asks the picker".to_owned());
    let name = BusName::try_from(PORTAL_BUS).map_err(|e| fdo::Error::Failed(e.to_string()))?;
    let owner = fdo::DBusProxy::new(connection)
        .await?
        .get_name_owner(name)
        .await
        .map_err(|_| denied())?;
    match header.sender() {
        Some(sender) if sender.as_str() == owner.as_str() => Ok(()),
        Some(_) | None => Err(denied()),
    }
}

#[interface(name = "org.quire.Shell1")]
impl PickerObject {
    async fn pick_capture_source(
        &self,
        app_label: String,
        kinds: u32,
        multiple: u32,
        cursor_modes: Vec<u32>,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<(Vec<(String, String)>, u32)> {
        from_transom(connection, &header).await?;
        let label: String = app_label.chars().take(MAX_LABEL).collect();
        let modes: Vec<u32> = cursor_modes.into_iter().take(MAX_MODES).collect();
        let ask = Ask::from_wire(&label, kinds, multiple, &modes)
            .map_err(|e| fdo::Error::InvalidArgs(e.to_string()))?;
        let (answer, answered) = oneshot::channel();
        let stopped = || fdo::Error::Failed("the picker has stopped".to_owned());
        self.calls
            .send(Call::Sources(SourceCall { ask, answer }))
            .map_err(|_| stopped())?;
        let reply = answered.await.map_err(|_| stopped())??;
        Ok((reply.sources, reply.cursor))
    }

    /// One colour from the screen, as the portal's `PickColor` asks it: red, green and blue in
    /// 0..1, or an empty array when the person cancelled.
    async fn pick_color(
        &self,
        app_label: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<Vec<(f64, f64, f64)>> {
        from_transom(connection, &header).await?;
        let ask = ColorAsk {
            app_label: app_label.chars().take(MAX_LABEL).collect(),
        };
        let (answer, answered) = oneshot::channel();
        let stopped = || fdo::Error::Failed("the picker has stopped".to_owned());
        self.calls
            .send(Call::Color(ColorCall { ask, answer }))
            .map_err(|_| stopped())?;
        let reply = answered.await.map_err(|_| stopped())??;
        Ok(reply.wire())
    }
}
