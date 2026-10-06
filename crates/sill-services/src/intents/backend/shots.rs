//! The file a screenshot was saved as. The tool saves it and the folder watch notices it, so the
//! path is read from the screenshots service's state once it holds a shot newer than the one
//! seen before the tool ran. When none appears in time (the person is still choosing, or the
//! capture went to the clipboard) the answer has no path.

use std::time::Duration;

use docket_core::{Invocation, Value};
use porter_core::{AppName, DataClass};
use prov::{Label, Labelled, Source};
use sill_model::screenshots::{ShotId, Shots};
use tokio::sync::watch;

/// How long to wait for the file to show up: well inside the action's `quick` latency.
const SHOT_WAIT: Duration = Duration::from_secs(3);

/// The saved path of the screenshot newer than `after`, labelled as the shell's own word about a
/// file name, or `None` when none was seen in time.
pub(super) async fn saved(
    shots: &mut watch::Receiver<Shots>,
    after: ShotId,
    inv: &Invocation,
) -> Option<Labelled<Value>> {
    let waited = tokio::time::timeout(SHOT_WAIT, shots.wait_for(|s| s.last_id > after)).await;
    let path = waited
        .ok()?
        .ok()?
        .current
        .as_ref()
        .map(|shot| shot.path.to_string_lossy().into_owned())?;
    let app = AppName::parse("org.quire.Shell").ok()?;
    let label = Label::untrusted(Source::App(app), DataClass::Screen, inv.space.clone());
    Some(Labelled::new(Value::Text(path), label))
}
