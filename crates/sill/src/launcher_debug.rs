//! `sill debug launcher-key KEY` and `sill debug launcher-rows`: what `dev/accept-launcher-v2.sh`
//! drives the open launcher with on the nested compositor, where there is no keyboard to type on.
//! Both go through the `LauncherLink`: a key the panel steps as if pressed, and the rows it shows
//! (clipboard titles already `<clipboard>`). Refused by a daemon built without `debug`.

use sill_ipc::{
    Failure, FailureKind, FakeLauncherKey, LauncherRows, ReportPreview, ReportPreviewKind,
    Response, RowsSection,
};
use sill_launcher::PreviewKind;
use sill_surfaces::surfaces::launcher::LauncherLink;
use sill_surfaces::surfaces::launcher::debug_link::{LinkKey, ShownRows};

/// Press `key` on the open launcher.
pub fn key(launcher: &LauncherLink, key: FakeLauncherKey) -> Response {
    if launcher.now().opening().is_none() {
        return not_open();
    }
    launcher.press(link_key(key));
    Response::Done
}

/// The open launcher's rows as shown.
pub fn rows(launcher: &LauncherLink) -> Response {
    match (launcher.now().opening(), launcher.rows()) {
        (Some(_), Some(rows)) => Response::LauncherRows(report(rows)),
        _ => not_open(),
    }
}

/// `key` in the surface's words.
pub fn link_key(key: FakeLauncherKey) -> LinkKey {
    match key {
        FakeLauncherKey::Up => LinkKey::Up,
        FakeLauncherKey::Down => LinkKey::Down,
        FakeLauncherKey::Enter => LinkKey::Enter,
        FakeLauncherKey::Escape => LinkKey::Escape,
        FakeLauncherKey::Tab => LinkKey::Tab,
        FakeLauncherKey::ShiftTab => LinkKey::ShiftTab,
        FakeLauncherKey::Actions => LinkKey::Actions,
        FakeLauncherKey::Space => LinkKey::Space,
        FakeLauncherKey::QuickLook => LinkKey::QuickLook,
        FakeLauncherKey::Right => LinkKey::Right,
        FakeLauncherKey::Left => LinkKey::Left,
        FakeLauncherKey::Reveal => LinkKey::Reveal,
    }
}

/// The panel's rows as the IPC reports them.
pub fn report(rows: ShownRows) -> LauncherRows {
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
        preview: rows.preview.map_or(ReportPreview::Hidden, |kind| {
            ReportPreview::Shown(preview_kind(kind))
        }),
    }
}

fn preview_kind(kind: PreviewKind) -> ReportPreviewKind {
    match kind {
        PreviewKind::None => ReportPreviewKind::None,
        PreviewKind::Image => ReportPreviewKind::Image,
        PreviewKind::Text => ReportPreviewKind::Text,
        PreviewKind::TextFile => ReportPreviewKind::TextFile,
        PreviewKind::Pdf => ReportPreviewKind::Pdf,
        PreviewKind::File => ReportPreviewKind::File,
        PreviewKind::App => ReportPreviewKind::App,
        PreviewKind::Clipboard => ReportPreviewKind::Clipboard,
        PreviewKind::Emoji => ReportPreviewKind::Emoji,
        PreviewKind::Web => ReportPreviewKind::Web,
    }
}

fn not_open() -> Response {
    Response::Failed(Failure {
        kind: FailureKind::Unsupported,
        message: "the launcher is not open".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use sill_ipc::{FakeLauncherKey, ReportPreview, ReportPreviewKind, Response};
    use sill_launcher::PreviewKind;
    use sill_surfaces::surfaces::launcher::LauncherLink;
    use sill_surfaces::surfaces::launcher::debug_link::{LinkKey, ShownRows, ShownRowsSection};

    use super::{key, link_key, report, rows};

    #[test]
    fn every_fake_key_has_its_link_key() {
        let cases = [
            (FakeLauncherKey::Space, LinkKey::Space),
            (FakeLauncherKey::QuickLook, LinkKey::QuickLook),
            (FakeLauncherKey::Reveal, LinkKey::Reveal),
            (FakeLauncherKey::ShiftTab, LinkKey::ShiftTab),
        ];
        for (fake, want) in cases {
            assert_eq!(link_key(fake), want);
        }
    }

    #[test]
    fn a_closed_launcher_answers_nothing_and_an_open_one_its_rows() {
        let link = LauncherLink::new();
        assert!(matches!(rows(&link), Response::Failed(_)));
        assert!(matches!(
            key(&link, FakeLauncherKey::Down),
            Response::Failed(_)
        ));
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
        assert_eq!(shown.preview, ReportPreview::Shown(ReportPreviewKind::Pdf));
        assert_eq!(key(&link, FakeLauncherKey::Down), Response::Done);
        let mut presses = link.subscribe_keys();
        assert_eq!(presses.borrow_and_update().key, Some(LinkKey::Down));
        let hidden = report(ShownRows::default());
        assert_eq!(hidden.preview, ReportPreview::Hidden);
    }
}
