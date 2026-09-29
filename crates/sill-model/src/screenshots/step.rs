//! The screenshots state and its one reducer: effects (open, trash, preview, run the tool) are
//! returned, never performed.

use std::path::PathBuf;

use super::{Shot, ShotEffect, ShotEvent, ShotId, ShotLeave, ShotPreview, Shots, Taking};

/// The one place [`Shots`] changes. `Appeared` gives the next id, makes it current (replacing
/// any other) with a pending preview and asks for one; the same path appearing again while it is
/// current is ignored (a watch sees both the create and the close). A preview answer for a shot
/// no longer current is dropped. `Left` of the current shot clears it and asks for `Open` or
/// `Trash` as the reason says; of any other id, nothing. `Take` while `Running` is ignored.
pub fn step(shots: Shots, event: ShotEvent) -> (Shots, Vec<ShotEffect>) {
    match event {
        ShotEvent::Appeared(path) => appeared(shots, path),
        ShotEvent::PreviewReady(id, preview) => (
            previewed(shots, id, ShotPreview::Ready(preview)),
            Vec::new(),
        ),
        ShotEvent::PreviewFailed(id) => (previewed(shots, id, ShotPreview::Original), Vec::new()),
        ShotEvent::Left(id, why) => left(shots, id, why),
        ShotEvent::Take(kind) => match shots.taking {
            Taking::Running(_) => (shots, Vec::new()),
            Taking::Idle => (
                Shots {
                    taking: Taking::Running(kind),
                    ..shots
                },
                vec![ShotEffect::RunTool(kind)],
            ),
        },
        ShotEvent::TakeEnded => (
            Shots {
                taking: Taking::Idle,
                ..shots
            },
            Vec::new(),
        ),
        ShotEvent::Watching(watching) => (Shots { watching, ..shots }, Vec::new()),
    }
}

fn appeared(shots: Shots, path: PathBuf) -> (Shots, Vec<ShotEffect>) {
    if shots.current.as_ref().is_some_and(|shot| shot.path == path) {
        return (shots, Vec::new());
    }
    let id = ShotId(shots.last_id.0 + 1);
    let effect = ShotEffect::MakePreview {
        id,
        from: path.clone(),
    };
    let current = Shot {
        id,
        path,
        preview: ShotPreview::Pending,
    };
    let next = Shots {
        current: Some(current),
        last_id: id,
        ..shots
    };
    (next, vec![effect])
}

fn previewed(shots: Shots, id: ShotId, preview: ShotPreview) -> Shots {
    match shots.current {
        Some(shot) if shot.id == id => Shots {
            current: Some(Shot { preview, ..shot }),
            ..shots
        },
        current => Shots { current, ..shots },
    }
}

fn left(shots: Shots, id: ShotId, why: ShotLeave) -> (Shots, Vec<ShotEffect>) {
    let Some(shot) = shots.current.clone().filter(|shot| shot.id == id) else {
        return (shots, Vec::new());
    };
    let effects = match why {
        ShotLeave::Opened => vec![ShotEffect::Open(shot.path)],
        ShotLeave::Trashed => vec![ShotEffect::Trash(shot.path)],
        ShotLeave::Kept | ShotLeave::Dragged | ShotLeave::Moved => Vec::new(),
    };
    (
        Shots {
            current: None,
            ..shots
        },
        effects,
    )
}
