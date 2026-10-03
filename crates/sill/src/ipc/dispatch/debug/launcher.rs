//! `sill debug launcher-rows`: what `dev/accept/launcher.sh` reads the open launcher with. It goes
//! through the `LauncherLink`: the rows the panel shows (clipboard titles already `<clipboard>`). Refused by a daemon built without `debug`.

use sill_ipc::response::Response;
use sill_ui_kit::links::LauncherLink;

use super::super::reply::unsupported;

/// The open launcher's rows as shown.
#[cfg(feature = "debug")]
pub fn rows(launcher: &LauncherLink) -> Response {
    match (launcher.now().opening(), launcher.rows()) {
        (Some(_), Some(rows)) => Response::LauncherRows(report(rows)),
        _ => not_open(),
    }
}

#[cfg(not(feature = "debug"))]
pub fn rows(_launcher: &LauncherLink) -> Response {
    unsupported("launcher rows need a daemon built with the debug feature")
}

#[cfg(feature = "debug")]
fn not_open() -> Response {
    unsupported("the launcher is not open")
}

/// The panel's rows as the wire reports them.
#[cfg(feature = "debug")]
fn report(rows: sill_ui_kit::links::ShownRows) -> sill_ipc::debug::LauncherRows {
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
    use sill_ipc::response::Response;
    use sill_launcher::PreviewKind;
    use sill_ui_kit::links::LauncherLink;
    use sill_ui_kit::links::{ShownRows, ShownRowsSection};

    use super::{report, rows};

    #[test]
    fn a_closed_launcher_answers_nothing_and_an_open_one_its_rows() {
        let link = LauncherLink::new();
        assert!(matches!(rows(&link), Response::Failed(_)));
        link.open(shell_host::OutputId(1), "m9".to_owned());
        link.show_rows(Some(ShownRows {
            query: "m9".to_owned(),
            sections: vec![ShownRowsSection {
                title: "Documents".to_owned(),
                rows: vec!["m9-report.pdf".to_owned()],
                hidden: 2,
            }],
            selected: 0,
            preview: Some(PreviewKind::File),
        }));
        let Response::LauncherRows(shown) = rows(&link) else {
            panic!("an open launcher answers its rows");
        };
        assert_eq!(shown.sections[0].hidden, 2);
        assert_eq!(shown.preview, Some(PreviewKind::File));
        assert_eq!(report(ShownRows::default()).preview, None);
    }
}
