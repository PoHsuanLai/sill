//! The screenshots task: on `Configure`, resolve the folders (`folders_of`, the Pictures folder
//! from the Pictures folder of [`Dirs`]), keep those that exist, watch them with `notify` (a file closed
//! after writing, or renamed into place, whose name `is_screenshot_name`, [`super::folder_watch`]), and
//! publish `Watching`; run each event and command through `reduce_shots` and carry out its
//! effects: `MakePreview` on the blocking pool ([`super::preview`]), `Open` (`xdg-open`),
//! `Trash` (`gio trash`), `RunTool` (`tool_command` into `save_folder`, a child whose exit is
//! `TakeEnded`). Ends when every receiver is gone.
//!
//! Passive until the first `Configure`: the policy starts `Off` (no folder, no tool).

use std::path::{Path, PathBuf};

use notify::RecommendedWatcher;
use tokio::sync::{mpsc, watch};

use super::reduce::{ShotEffect, ShotEvent, Shots, Watching, reduce_shots};
use super::tool::{ShotKind, save_folder, tool_command};
use super::{ShotCommand, ShotPolicy, folders_of};
use crate::env::Dirs;

/// The task's own state beside the published [`Shots`].
struct Live {
    tx: watch::Sender<Shots>,
    events: mpsc::UnboundedSender<ShotEvent>,
    policy: Option<ShotPolicy>,
    folders: Vec<PathBuf>,
    /// Held so the watch lives; replaced on every new policy.
    watcher: Option<RecommendedWatcher>,
    /// Pictures and home (the folders), and the cache (the previews).
    dirs: Dirs,
}

pub(super) async fn run(
    tx: watch::Sender<Shots>,
    mut commands: mpsc::UnboundedReceiver<ShotCommand>,
    dirs: Dirs,
) {
    let (events, mut heard) = mpsc::unbounded_channel();
    let mut live = Live {
        tx,
        events,
        policy: None,
        folders: Vec::new(),
        watcher: None,
        dirs,
    };
    loop {
        let event = tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command),
                None => return,
            },
            Some(event) = heard.recv() => Some(event),
            () = live.tx.closed() => return,
        };
        if let Some(event) = event {
            live.apply(event);
        }
    }
}

impl Live {
    /// The event a command makes, if any.
    fn command(&mut self, command: ShotCommand) -> Option<ShotEvent> {
        match command {
            ShotCommand::Configure(policy) => self.configure(policy),
            ShotCommand::Take(kind) => Some(ShotEvent::Take(kind)),
            ShotCommand::Announce(path) => Some(ShotEvent::Appeared(path)),
            ShotCommand::Leave(id, why) => Some(ShotEvent::Left(id, why)),
        }
    }

    /// Watch `policy`'s folders (dropping the old watch first); nothing when it is the same.
    fn configure(&mut self, policy: ShotPolicy) -> Option<ShotEvent> {
        if self.policy.as_ref() == Some(&policy) {
            return None;
        }
        self.watcher = None;
        self.folders = folders_of(&policy.folders, self.dirs.pictures.as_deref());
        let (watcher, watching) = super::folder_watch::start(&self.folders, self.events.clone());
        self.watcher = watcher;
        self.policy = Some(policy);
        Some(ShotEvent::Watching(watching))
    }

    fn apply(&mut self, event: ShotEvent) {
        let (next, effects) = reduce_shots(self.tx.borrow().clone(), event);
        self.tx.send_if_modified(|now| {
            let changed = *now != next;
            *now = next;
            changed
        });
        for effect in effects {
            self.perform(effect);
        }
    }

    fn perform(&self, effect: ShotEffect) {
        match effect {
            ShotEffect::MakePreview { id, from } => {
                let events = self.events.clone();
                let cache = super::preview::cache_in(&self.dirs);
                tokio::spawn(async move {
                    let made =
                        tokio::task::spawn_blocking(move || super::preview::make_in(&from, &cache))
                            .await
                            .ok()
                            .and_then(Result::ok);
                    let _ = events.send(match made {
                        Some(preview) => ShotEvent::PreviewReady(id, preview),
                        None => ShotEvent::PreviewFailed(id),
                    });
                });
            }
            ShotEffect::Open(path) => super::child::detach("xdg-open", &[path.as_os_str()]),
            ShotEffect::Trash(path) => {
                super::child::detach("gio", &["trash".as_ref(), path.as_os_str()]);
            }
            ShotEffect::RunTool(kind) => self.run_tool(kind),
        }
    }

    /// Run the tool for `kind` and answer `TakeEnded` at its exit (at once when there is none).
    fn run_tool(&self, kind: ShotKind) {
        let folder = save_folder(&self.folders)
            .map(Path::to_path_buf)
            .or_else(|| self.dirs.pictures.clone())
            .unwrap_or_else(|| self.dirs.home.clone());
        let tool = self.policy.clone().unwrap_or_default().tool;
        let events = self.events.clone();
        match tool_command(&tool, kind, &folder) {
            Some(line) => super::child::run_then(line, move || {
                let _ = events.send(ShotEvent::TakeEnded);
            }),
            None => {
                let _ = events.send(ShotEvent::TakeEnded);
            }
        }
    }
}

/// What a `Watching` says for `folders` that exist (none: `Off`).
pub(super) fn watching_of(folders: Vec<PathBuf>) -> Watching {
    match folders.is_empty() {
        true => Watching::Off,
        false => Watching::Folders(folders),
    }
}
