//! `org.quire.Shell1`, as the shell exports it on the seat. The object checks who is calling and
//! hands the call to the loop, which decides, and waits for the loop's answer.

use std::sync::Arc;

use peephole_bus::peephole_core::roles;
use sill_model::capture_pick::{
    Ask, ColorAsk, ColorReply, PermissionAsk, PermissionOutcome, Remembered, Reply,
};
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use zbus::fdo;
use zbus::interface;
use zbus::message::Header;

use crate::callers::{Claim, Peers, admit};

/// The portal backend's name: transom owns it, and only its owner, holding the portal role, may
/// ask the person.
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
    /// A yes-or-no question about an app.
    Permission(PermissionCall),
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

/// A permission question, and where its answer goes.
#[derive(Debug)]
pub(super) struct PermissionCall {
    pub ask: PermissionAsk,
    pub answer: oneshot::Sender<fdo::Result<PermissionOutcome>>,
}

/// The picker object.
pub(super) struct PickerObject {
    pub calls: UnboundedSender<Call>,
    /// Tells the callers on the seat's connection apart.
    pub peers: Arc<Peers>,
}

impl PickerObject {
    /// Lets the call with `header` through when it is the portal backend: the program the table
    /// names as `portal`, which owns the portal backend's name.
    async fn admit_transom(&self, header: &Header<'_>) -> fdo::Result<()> {
        let claims = [Claim {
            name: PORTAL_BUS,
            role: roles::PORTAL,
            tag: (),
        }];
        admit(&self.peers, header, &claims).await.map(|_| ())
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
        remembered: u32,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<(Vec<(String, String)>, u32)> {
        self.admit_transom(&header).await?;
        let label: String = app_label.chars().take(MAX_LABEL).collect();
        let modes: Vec<u32> = cursor_modes.into_iter().take(MAX_MODES).collect();
        let ask = Ask::from_wire(&label, kinds, multiple, &modes)
            .map_err(|e| fdo::Error::InvalidArgs(e.to_string()))?
            .remembering(Remembered::from_wire(remembered));
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
    ) -> fdo::Result<Vec<(f64, f64, f64)>> {
        self.admit_transom(&header).await?;
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

    /// A yes-or-no question about an app, as the portal's Access backend asks it: the answer is
    /// 0 when the person pressed the yes button, 1 the no button, 2 when the sheet went away
    /// without an answer. The texts are cleaned and cut by [`PermissionAsk::from_wire`].
    #[allow(clippy::too_many_arguments)]
    async fn ask_permission(
        &self,
        app_label: String,
        title: String,
        subtitle: String,
        body: String,
        grant_label: String,
        deny_label: String,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<u32> {
        self.admit_transom(&header).await?;
        let ask = PermissionAsk::from_wire(
            &app_label,
            &title,
            &subtitle,
            &body,
            &grant_label,
            &deny_label,
        );
        let (answer, answered) = oneshot::channel();
        let stopped = || fdo::Error::Failed("the picker has stopped".to_owned());
        self.calls
            .send(Call::Permission(PermissionCall { ask, answer }))
            .map_err(|_| stopped())?;
        let outcome = answered.await.map_err(|_| stopped())??;
        Ok(outcome.wire())
    }
}
