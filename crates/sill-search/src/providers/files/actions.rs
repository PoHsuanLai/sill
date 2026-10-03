//! A file row's actions: the list the viewer and the launcher share (`anyview_core::actions_for`),
//! the ones that appear in the launcher (`reach`), each with its keys (`shortcut`), and what
//! choosing it does. The launcher carries out the actions it can; one it cannot yet (Open With,
//! Share, Rename...) is left off the row, not drawn dead.
//!
//! [`carried`] is the one match on [`FileAction`] here: it says, for each, whether the launcher
//! does it and what it asks for, so the row's menu and `Provider::activate` cannot disagree.

use anyview_core::{
    FileAction, FormatKind, Mime as CoreMime, Reach, actions_for, kind_of_mime, reach, shortcut,
};
use ds_core::vocab::{Shortcut, ShortcutKey};
use ds_core::word::Word;
use sill_launcher::{
    Action, ActionId, ActionKeys, Activation, Chord, ChordKey, FileFacts, Generation, Mime,
    Modifier, OpenTarget, ViewerOpen,
};

/// What the launcher does for one action on one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    /// The words on the row's menu.
    pub label: &'static str,
    /// What choosing it asks of the daemon.
    pub activation: Activation,
}

/// What the launcher does for `action` on `file`, or `None` when it does not do it (yet).
pub fn carried(action: FileAction, file: &FileFacts) -> Option<Carried> {
    let path = file.path.clone();
    let (label, activation) = match action {
        FileAction::Open => ("Open", open(file)),
        FileAction::RevealInFolder => (
            "Reveal in Files",
            Activation::Open {
                target: OpenTarget::Reveal(path),
            },
        ),
        FileAction::CopyPath => (
            "Copy Path",
            Activation::Copy {
                text: path.to_string_lossy().into_owned(),
            },
        ),
        FileAction::PlayInBackground => (
            "Play",
            Activation::Open {
                target: OpenTarget::Play(path),
            },
        ),
        FileAction::OpenWith
        | FileAction::CopyFile
        | FileAction::Share
        | FileAction::Rename
        | FileAction::Duplicate
        | FileAction::MoveToTrash
        | FileAction::Print
        | FileAction::Export
        | FileAction::SaveCopy
        | FileAction::RevertTo
        | FileAction::RotateLeft
        | FileAction::RotateRight
        | FileAction::FlipHorizontal
        | FileAction::FlipVertical
        | FileAction::PlayInMiniWindow
        | FileAction::ConvertTo => return None,
    };
    Some(Carried { label, activation })
}

/// Open: a folder in the file manager, everything else in the viewer. The results are the
/// panel's to add when it knows the list ([`with_results`](sill_launcher::ViewerOpen)).
fn open(file: &FileFacts) -> Activation {
    let target = match file.mime.0.as_str() {
        Mime::FOLDER => OpenTarget::File(file.path.clone()),
        _ => OpenTarget::Viewer(ViewerOpen {
            file: file.path.clone(),
            search: Generation::default(),
            results: Vec::new(),
        }),
    };
    Activation::Open { target }
}

/// What kind of file `file` is, as far as its media type says (`Other` for a type that is not
/// one).
pub fn kind_of(file: &FileFacts) -> FormatKind {
    CoreMime::parse(&file.mime.0).map_or(FormatKind::Other, |mime| kind_of_mime(&mime))
}

/// The actions on `file`'s row, in the shared list's order: those the launcher shows
/// (`Reach::Launcher` or `Reach::Both`) and carries out.
pub fn row_actions(file: &FileFacts) -> Vec<Action> {
    actions_for(kind_of(file))
        .iter()
        .copied()
        .filter(|action| shows_in_launcher(reach(*action)))
        .filter_map(|action| {
            let carried = carried(action, file)?;
            Some(Action {
                id: ActionId(action.slug().to_owned()),
                label: carried.label.to_owned(),
                keys: shortcut(action)
                    .and_then(|keys| chord_of(&keys))
                    .map_or(ActionKeys::None, ActionKeys::Chord),
            })
        })
        .collect()
}

/// Whether an action of `reach` is offered by the launcher.
fn shows_in_launcher(reach: Reach) -> bool {
    match reach {
        Reach::Launcher | Reach::Both => true,
        Reach::Viewer => false,
    }
}

/// What choosing the action `id` on `file` does, or nothing for an id that is not one of its
/// carried actions.
pub fn activation_of(id: &ActionId, file: &FileFacts) -> Activation {
    FileAction::parse(&id.0)
        .and_then(|action| carried(action, file))
        .map_or(Activation::Nothing, |carried| carried.activation)
}

/// `keys` as a chord of the launcher's, in Mac terms; `None` when it ends on a key a chord has
/// no word for.
pub fn chord_of(keys: &Shortcut) -> Option<Chord> {
    let mut mods = Vec::new();
    let mut key = None;
    for pressed in keys.keys() {
        match pressed {
            ShortcutKey::Ctrl => mods.push(Modifier::Control),
            ShortcutKey::Alt => mods.push(Modifier::Option),
            ShortcutKey::Shift => mods.push(Modifier::Shift),
            ShortcutKey::Super => mods.push(Modifier::Command),
            ShortcutKey::Char(c) => key = Some(ChordKey::Char(c.to_ascii_lowercase())),
            ShortcutKey::Space => key = Some(ChordKey::Space),
            ShortcutKey::Enter => key = Some(ChordKey::Enter),
            ShortcutKey::Left => key = Some(ChordKey::Left),
            ShortcutKey::Right => key = Some(ChordKey::Right),
            ShortcutKey::Backspace => key = Some(ChordKey::Backspace),
            ShortcutKey::Escape
            | ShortcutKey::Tab
            | ShortcutKey::Up
            | ShortcutKey::Down
            | ShortcutKey::Home
            | ShortcutKey::End
            | ShortcutKey::Delete
            | ShortcutKey::PageUp
            | ShortcutKey::PageDown
            | ShortcutKey::Insert
            | ShortcutKey::ContextMenu => return None,
        }
    }
    key.map(|key| Chord::new(mods, key))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use anyview_core::FileAction;
    use ds_core::standard_action::StandardAction;
    use sill_launcher::{ActionKeys, Activation, Mime, OpenTarget, UnixTime, chords};

    use super::*;

    fn file(path: &str, mime: &str) -> FileFacts {
        FileFacts {
            path: PathBuf::from(path),
            mime: Mime(mime.to_owned()),
            modified: UnixTime(0),
        }
    }

    fn ids(file: &FileFacts) -> Vec<String> {
        row_actions(file).into_iter().map(|a| a.id.0).collect()
    }

    #[test]
    fn each_kind_lists_the_shared_actions_the_launcher_shows_and_carries() {
        // what the row is, the ids on its menu in order
        let cases = [
            (
                "a pdf",
                file("/h/a.pdf", "application/pdf"),
                vec!["open", "reveal_in_folder", "copy_path"],
            ),
            (
                "a photo",
                file("/h/a.png", "image/png"),
                vec!["open", "reveal_in_folder", "copy_path"],
            ),
            (
                "a song",
                file("/h/a.flac", "audio/flac"),
                vec![
                    "open",
                    "reveal_in_folder",
                    "copy_path",
                    "play_in_background",
                ],
            ),
            (
                "a video",
                file("/h/a.mkv", "video/x-matroska"),
                vec!["open", "reveal_in_folder", "copy_path"],
            ),
            (
                "a folder",
                file("/h", Mime::FOLDER),
                vec!["open", "reveal_in_folder", "copy_path"],
            ),
            (
                "an unknown type",
                file("/h/a.xyz", "application/x-unheard-of"),
                vec!["open", "reveal_in_folder", "copy_path"],
            ),
            (
                "a type that is not one",
                file("/h/a", "not a media type"),
                vec!["open", "reveal_in_folder", "copy_path"],
            ),
        ];
        for (name, file, want) in cases {
            assert_eq!(ids(&file), want, "{name}");
        }
    }

    #[test]
    fn a_viewer_only_action_is_never_on_a_launcher_row() {
        let pdf = file("/h/a.pdf", "application/pdf");
        // The shared list holds them for a pdf; the row does not.
        for viewer_only in [
            FileAction::Print,
            FileAction::Export,
            FileAction::RotateLeft,
        ] {
            assert!(
                actions_for(FormatKind::Pdf).contains(&viewer_only),
                "{viewer_only:?}"
            );
            assert!(
                !ids(&pdf).contains(&viewer_only.slug().to_owned()),
                "{viewer_only:?}"
            );
        }
    }

    #[test]
    fn an_action_wears_the_keys_the_shared_table_gives_it() {
        let song = file("/h/a.flac", "audio/flac");
        let keys: Vec<(String, ActionKeys)> = row_actions(&song)
            .into_iter()
            .map(|action| (action.id.0, action.keys))
            .collect();
        assert_eq!(
            keys,
            [
                (
                    "open".to_owned(),
                    ActionKeys::Chord(Chord::command(ChordKey::Char('o')))
                ),
                (
                    "reveal_in_folder".to_owned(),
                    ActionKeys::Chord(chords::reveal())
                ),
                (
                    "copy_path".to_owned(),
                    ActionKeys::Chord(Chord::new(
                        vec![Modifier::Option, Modifier::Command],
                        ChordKey::Char('c')
                    ))
                ),
                ("play_in_background".to_owned(), ActionKeys::None),
            ]
        );
        // The row's chords are the table's: ask it for the standard ones.
        assert_eq!(
            shortcut(FileAction::Open),
            Some(Shortcut::standard(StandardAction::Open))
        );
    }

    #[test]
    fn choosing_an_action_asks_for_what_it_says() {
        let pdf = file("/h/a b.pdf", "application/pdf");
        let song = file("/h/a.flac", "audio/flac");
        let folder = file("/h/docs", Mime::FOLDER);
        let id = |slug: &str| ActionId(slug.to_owned());
        let viewer = |path: &str| Activation::Open {
            target: OpenTarget::Viewer(ViewerOpen {
                file: PathBuf::from(path),
                search: Generation::default(),
                results: Vec::new(),
            }),
        };
        let open = |target| Activation::Open { target };
        let cases = [
            ("open a file", id("open"), &pdf, viewer("/h/a b.pdf")),
            (
                "open a folder",
                id("open"),
                &folder,
                open(OpenTarget::File(PathBuf::from("/h/docs"))),
            ),
            (
                "reveal",
                id("reveal_in_folder"),
                &pdf,
                open(OpenTarget::Reveal(PathBuf::from("/h/a b.pdf"))),
            ),
            (
                "copy the path",
                id("copy_path"),
                &pdf,
                Activation::Copy {
                    text: "/h/a b.pdf".to_owned(),
                },
            ),
            (
                "play",
                id("play_in_background"),
                &song,
                open(OpenTarget::Play(PathBuf::from("/h/a.flac"))),
            ),
            (
                "an action not carried",
                id("share"),
                &pdf,
                Activation::Nothing,
            ),
            (
                "a word that is no action",
                id("burn"),
                &pdf,
                Activation::Nothing,
            ),
        ];
        for (name, action, file, want) in cases {
            assert_eq!(activation_of(&action, file), want, "{name}");
        }
    }

    #[test]
    fn a_shortcut_becomes_a_chord_in_the_macs_order() {
        let cases = [
            (
                vec![ShortcutKey::Super, ShortcutKey::Char('R')],
                Some(chords::reveal()),
            ),
            (
                vec![ShortcutKey::Char('c'), ShortcutKey::Super, ShortcutKey::Alt],
                Some(Chord::new(
                    vec![Modifier::Command, Modifier::Option],
                    ChordKey::Char('c'),
                )),
            ),
            (
                vec![ShortcutKey::Super, ShortcutKey::Backspace],
                Some(chords::forget()),
            ),
            (vec![ShortcutKey::Super, ShortcutKey::Up], None),
            (vec![ShortcutKey::Super], None),
        ];
        for (keys, want) in cases {
            assert_eq!(chord_of(&Shortcut(keys.clone())), want, "{keys:?}");
        }
    }
}
