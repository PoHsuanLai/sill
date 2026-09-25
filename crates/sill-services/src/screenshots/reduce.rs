//! The screenshots state and its one reducer: effects (open, trash, preview, run the tool) are
//! returned, never performed.

use std::path::PathBuf;

use super::tool::ShotKind;

/// A screenshot's id, counting up from 1 for the daemon's life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ShotId(pub u64);

/// The small image the thumbnail draws.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShotPreview {
    /// Being made: the thumbnail waits for it (a few tens of ms for a 4K PNG).
    Pending,
    /// A downscaled PNG in `$XDG_CACHE_HOME/sill/shots/` ([`super::preview_name`]).
    Ready(PathBuf),
    /// The preview could not be made: draw the file itself.
    Original,
}

/// The screenshot the thumbnail shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shot {
    pub id: ShotId,
    /// The file, as saved.
    pub path: PathBuf,
    pub preview: ShotPreview,
}

/// Why a thumbnail went away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShotLeave {
    /// Clicked: the file opens in its default app (`xdg-open`).
    Opened,
    /// Swiped away, or its hold ran out: the file stays where it was saved.
    Kept,
    /// Its Delete action: the file goes to the trash.
    Trashed,
    /// Dragged out and dropped somewhere (shell-host G4's drag source): the file stays.
    Dragged,
}

/// What the folders' watch is doing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Watching {
    /// Nothing: passive, or `screenshot.folders = "off"`.
    #[default]
    Off,
    /// These folders (the ones that exist).
    Folders(Vec<PathBuf>),
    /// The watch could not start; the reason, for `sill debug` and the log.
    Failed(String),
}

/// Whether the tool is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Taking {
    #[default]
    Idle,
    Running(ShotKind),
}

/// What the thumbnail follows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Shots {
    /// The newest screenshot not yet dismissed; a newer one replaces it (its file stays).
    pub current: Option<Shot>,
    pub watching: Watching,
    pub taking: Taking,
    /// The last id given.
    pub last_id: ShotId,
}

/// What happens to the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShotEvent {
    /// A new screenshot file (the watch, or `Announce`).
    Appeared(PathBuf),
    PreviewReady(ShotId, PathBuf),
    PreviewFailed(ShotId),
    /// The thumbnail of `id` went away.
    Left(ShotId, ShotLeave),
    /// `Take` asked for a tool run.
    Take(ShotKind),
    /// The tool exited (whatever it saved arrives as `Appeared`).
    TakeEnded,
    Watching(Watching),
}

/// What the reducer asks the task to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShotEffect {
    /// Downscale `from` into the cache and answer `PreviewReady` or `PreviewFailed`.
    MakePreview { id: ShotId, from: PathBuf },
    /// `xdg-open` the file.
    Open(PathBuf),
    /// Move the file to the trash (`gio trash`).
    Trash(PathBuf),
    /// Run the tool for this kind, and answer `TakeEnded` when it exits.
    RunTool(ShotKind),
}

/// The one place [`Shots`] changes. `Appeared` gives the next id, makes it current (replacing
/// any other) with a pending preview and asks for one; the same path appearing again while it is
/// current is ignored (a watch sees both the create and the close). A preview answer for a shot
/// no longer current is dropped. `Left` of the current shot clears it and asks for `Open` or
/// `Trash` as the reason says; of any other id, nothing. `Take` while `Running` is ignored.
pub fn reduce_shots(shots: Shots, event: ShotEvent) -> (Shots, Vec<ShotEffect>) {
    let _ = (shots, event);
    todo!("M10 fill: c-screenshot")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        Shot, ShotEffect, ShotEvent, ShotId, ShotLeave, ShotPreview, Shots, Taking, reduce_shots,
    };
    use crate::screenshots::tool::ShotKind;

    fn shot(id: u64, name: &str, preview: ShotPreview) -> Shot {
        Shot {
            id: ShotId(id),
            path: PathBuf::from(format!("/tmp/shots/{name}")),
            preview,
        }
    }

    fn path(name: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/shots/{name}"))
    }

    #[test]
    #[ignore = "M10 fill: c-screenshot"]
    fn a_screenshot_arrives_is_previewed_and_leaves() {
        let (one, fx) = reduce_shots(Shots::default(), ShotEvent::Appeared(path("a.png")));
        assert_eq!(one.current, Some(shot(1, "a.png", ShotPreview::Pending)));
        assert_eq!(
            fx,
            vec![ShotEffect::MakePreview {
                id: ShotId(1),
                from: path("a.png")
            }]
        );
        let (again, fx) = reduce_shots(one.clone(), ShotEvent::Appeared(path("a.png")));
        assert_eq!(
            (again, fx),
            (one.clone(), Vec::new()),
            "the same file twice"
        );
        let (ready, _) = reduce_shots(one, ShotEvent::PreviewReady(ShotId(1), path("p.png")));
        assert_eq!(
            ready.current,
            Some(shot(1, "a.png", ShotPreview::Ready(path("p.png"))))
        );
        let cases = [
            (ShotLeave::Opened, vec![ShotEffect::Open(path("a.png"))]),
            (ShotLeave::Trashed, vec![ShotEffect::Trash(path("a.png"))]),
            (ShotLeave::Kept, Vec::new()),
            (ShotLeave::Dragged, Vec::new()),
        ];
        for (why, want) in cases {
            let (gone, fx) = reduce_shots(ready.clone(), ShotEvent::Left(ShotId(1), why));
            assert_eq!(gone.current, None, "{why:?}");
            assert_eq!(fx, want, "{why:?}");
        }
        let (stale, fx) =
            reduce_shots(ready.clone(), ShotEvent::Left(ShotId(9), ShotLeave::Opened));
        assert_eq!((stale, fx), (ready, Vec::new()));
    }

    #[test]
    #[ignore = "M10 fill: c-screenshot"]
    fn a_newer_screenshot_replaces_the_current_one_and_stale_previews_drop() {
        let (one, _) = reduce_shots(Shots::default(), ShotEvent::Appeared(path("a.png")));
        let (two, _) = reduce_shots(one, ShotEvent::Appeared(path("b.png")));
        assert_eq!(two.current, Some(shot(2, "b.png", ShotPreview::Pending)));
        let (same, fx) = reduce_shots(two.clone(), ShotEvent::PreviewReady(ShotId(1), path("p")));
        assert_eq!((same, fx), (two.clone(), Vec::new()));
        let (failed, _) = reduce_shots(two, ShotEvent::PreviewFailed(ShotId(2)));
        assert_eq!(
            failed.current,
            Some(shot(2, "b.png", ShotPreview::Original))
        );
    }

    #[test]
    #[ignore = "M10 fill: c-screenshot"]
    fn one_tool_run_at_a_time() {
        let (running, fx) = reduce_shots(Shots::default(), ShotEvent::Take(ShotKind::Area));
        assert_eq!(running.taking, Taking::Running(ShotKind::Area));
        assert_eq!(fx, vec![ShotEffect::RunTool(ShotKind::Area)]);
        let (still, fx) = reduce_shots(running.clone(), ShotEvent::Take(ShotKind::Screen));
        assert_eq!((still, fx), (running.clone(), Vec::new()));
        let (idle, _) = reduce_shots(running, ShotEvent::TakeEnded);
        assert_eq!(idle.taking, Taking::Idle);
    }
}
