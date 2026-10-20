//! The screenshots task: on `Configure`, resolve the folders (`folders_of`, the Pictures folder
//! from the Pictures folder of [`Dirs`]), keep those that exist, watch them with
//! [`crate::debounced`] (a burst of writes settles into one look at the folders for a name that
//! `is_screenshot_name`, [`super::folder_watch`]), and publish `Watching`; run each event and
//! command through `step` and carry out its effects: `MakePreview` on the blocking pool ([`super::preview`]), `Open` (`xdg-open`),
//! `Trash` (`gio trash`), `RunTool` (`tool_command` into `save_folder`, a child whose exit is
//! `TakeEnded`). Ends when every receiver is gone.
//!
//! Passive until the first `Configure`: the policy starts `Off` (no folder, no tool).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sill_model::screenshots::{
    ShotCommand, ShotEffect, ShotEvent, ShotKind, ShotPolicy, Shots, Watching, folders_of,
    save_folder, step, tool_command,
};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;

use crate::env::{Children, Dirs};
use crate::machine::Machine;

/// The task's own state beside the published [`Shots`].
struct Live {
    machine: Machine<Shots, ShotEvent, ShotEffect>,
    events: mpsc::UnboundedSender<ShotEvent>,
    policy: Option<ShotPolicy>,
    folders: Vec<PathBuf>,
    /// Held so the watch lives; replaced on every new policy.
    watcher: JoinSet<()>,
    /// Pictures and home (the folders), and the cache (the previews).
    dirs: Dirs,
    /// Starts the opener and the trash.
    children: Arc<dyn Children>,
}

pub(crate) async fn run(
    tx: watch::Sender<Shots>,
    mut commands: mpsc::UnboundedReceiver<ShotCommand>,
    dirs: Dirs,
    children: Arc<dyn Children>,
) {
    let (events, mut heard) = mpsc::unbounded_channel();
    let mut live = Live {
        machine: Machine::new(tx, step),
        events,
        policy: None,
        folders: Vec::new(),
        watcher: JoinSet::new(),
        dirs,
        children,
    };
    loop {
        let event = tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command),
                None => return,
            },
            Some(event) = heard.recv() => Some(event),
            () = live.machine.closed() => return,
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
        self.watcher = JoinSet::new();
        self.folders = folders_of(&policy.folders, self.dirs.pictures.as_deref());
        let (watcher, watching) = super::folder_watch::start(&self.folders, self.events.clone());
        self.watcher = watcher;
        self.policy = Some(policy);
        Some(ShotEvent::Watching(watching))
    }

    fn apply(&mut self, event: ShotEvent) {
        for effect in self.machine.feed(event) {
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
            ShotEffect::Open(path) => {
                super::child::detach(self.children.as_ref(), "xdg-open", &[path.as_os_str()])
            }
            ShotEffect::Trash(path) => {
                super::child::detach(
                    self.children.as_ref(),
                    "gio",
                    &["trash".as_ref(), path.as_os_str()],
                );
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
pub(crate) fn watching_of(folders: Vec<PathBuf>) -> Watching {
    match folders.is_empty() {
        true => Watching::Off,
        false => Watching::Folders(folders),
    }
}
