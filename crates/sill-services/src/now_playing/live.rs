//! MPRIS over the session bus: finds every `org.mpris.MediaPlayer2.*` name, follows their
//! `PropertiesChanged` and `Seeked`, fetches remote art into the cache, feeds every change
//! through [`super::reduce_media`], and carries out [`MediaCommand`]s.

use tokio::sync::{mpsc, watch};

use super::MediaCommand;
use super::model::Media;

/// Runs until `commands` closes. Without a session bus, `tx` stays at its default.
///
/// M5 freeze stub, filled by M5 agent (b) svc-media-wifi-focus (FINDINGS "M5 freeze"): until
/// then it returns at once, so the service is inert.
pub(super) async fn run(tx: watch::Sender<Media>, commands: mpsc::UnboundedReceiver<MediaCommand>) {
    let _ = (tx, commands);
}
