//! `sill debug launcher-key KEY` and `sill debug launcher-rows`: what `dev/accept-launcher-v2.sh`
//! drives the open launcher with on the nested compositor, where there is no keyboard to type
//! on. Both go through the `LauncherLink`: a key the panel steps as if pressed, and the rows it
//! shows (clipboard titles already `<clipboard>`). Refused by a daemon built without `debug`.

use sill_ipc::debug::LauncherKey;
use sill_ipc::response::Response;
use sill_surfaces::surfaces::launcher::LauncherLink;

use super::super::reply::unsupported;

/// Press `key` on the open launcher.
#[cfg(feature = "debug")]
pub fn key(launcher: &LauncherLink, key: LauncherKey) -> Response {
    if launcher.now().opening().is_none() {
        return not_open();
    }
    launcher.press(link_key(key));
    Response::Done
}

/// The open launcher's rows as shown.
#[cfg(feature = "debug")]
pub fn rows(launcher: &LauncherLink) -> Response {
    match (launcher.now().opening(), launcher.rows()) {
        (Some(_), Some(rows)) => Response::LauncherRows(report(rows)),
        _ => not_open(),
    }
}

#[cfg(not(feature = "debug"))]
pub fn key(_launcher: &LauncherLink, _key: LauncherKey) -> Response {
    unsupported("launcher keys need a daemon built with the debug feature")
}

#[cfg(not(feature = "debug"))]
pub fn rows(_launcher: &LauncherLink) -> Response {
    unsupported("launcher rows need a daemon built with the debug feature")
}

#[cfg(feature = "debug")]
fn not_open() -> Response {
    unsupported("the launcher is not open")
}

/// `key` in the surface's words.
#[cfg(feature = "debug")]
fn link_key(key: LauncherKey) -> sill_surfaces::surfaces::launcher::debug_link::LinkKey {
    use sill_surfaces::surfaces::launcher::debug_link::LinkKey;
    match key {
        LauncherKey::Up => LinkKey::Up,
        LauncherKey::Down => LinkKey::Down,
        LauncherKey::Enter => LinkKey::Enter,
        LauncherKey::Escape => LinkKey::Escape,
        LauncherKey::Tab => LinkKey::Tab,
        LauncherKey::ShiftTab => LinkKey::ShiftTab,
        LauncherKey::Actions => LinkKey::Actions,
        LauncherKey::Space => LinkKey::Space,
        LauncherKey::QuickLook => LinkKey::QuickLook,
        LauncherKey::Right => LinkKey::Right,
        LauncherKey::Left => LinkKey::Left,
        LauncherKey::Reveal => LinkKey::Reveal,
    }
}

/// The panel's rows as the wire reports them.
#[cfg(feature = "debug")]
fn report(
    rows: sill_surfaces::surfaces::launcher::debug_link::ShownRows,
) -> sill_ipc::debug::LauncherRows {
    use sill_ipc::debug::{LauncherRows, RowsSection};
    LauncherRows {
        query: rows.query,
        sections: rows
            .sections
            .into_iter()
            .map(|section| RowsSection {
                title: section.title,
                rows: section.rows,
                hidden: section.hidden,
            })
            .collect(),
        selected: rows.selected,
        preview: rows.preview,
    }
}

#[cfg(all(test, feature = "debug"))]
mod tests {
    use sill_ipc::debug::LauncherKey;
    use sill_ipc::response::Response;
    use sill_launcher::PreviewKind;
    use sill_surfaces::surfaces::launcher::LauncherLink;
    use sill_surfaces::surfaces::launcher::debug_link::{LinkKey, ShownRows, ShownRowsSection};

    use super::{key, link_key, report, rows};

    #[test]
    fn every_key_has_its_link_key() {
        let cases = [
            (LauncherKey::Space, LinkKey::Space),
            (LauncherKey::QuickLook, LinkKey::QuickLook),
            (LauncherKey::Reveal, LinkKey::Reveal),
            (LauncherKey::ShiftTab, LinkKey::ShiftTab),
        ];
        for (wire, want) in cases {
            assert_eq!(link_key(wire), want);
        }
    }

    #[test]
    fn a_closed_launcher_answers_nothing_and_an_open_one_its_rows() {
        let link = LauncherLink::new();
        assert!(matches!(rows(&link), Response::Failed(_)));
        assert!(matches!(key(&link, LauncherKey::Down), Response::Failed(_)));
        link.open(shell_host::OutputId(1), "m9".to_owned());
        link.show_rows(Some(ShownRows {
            query: "m9".to_owned(),
            sections: vec![ShownRowsSection {
                title: "Documents".to_owned(),
                rows: vec!["m9-report.pdf".to_owned()],
                hidden: 2,
            }],
            selected: 0,
            preview: Some(PreviewKind::Pdf),
        }));
        let Response::LauncherRows(shown) = rows(&link) else {
            panic!("an open launcher answers its rows");
        };
        assert_eq!(shown.sections[0].hidden, 2);
        assert_eq!(shown.preview, Some(PreviewKind::Pdf));
        assert_eq!(key(&link, LauncherKey::Down), Response::Done);
        let mut presses = link.subscribe_keys();
        assert_eq!(presses.borrow_and_update().key, Some(LinkKey::Down));
        assert_eq!(report(ShownRows::default()).preview, None);
    }
}
