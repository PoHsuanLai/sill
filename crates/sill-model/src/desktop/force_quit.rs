//! What the Force Quit Applications panel lists: the running apps the person can see, in the
//! order they were last used, each marked when it does not answer. Pure.

use casement_protocols::vocab::{AppLifeKind, Responsiveness};

use super::{AppView, DesktopView};

/// Whether an app answers the desktop's pings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Responding,
    /// The panel says "(not responding)" beside its name.
    NotResponding,
}

/// One app in the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceQuitRow {
    /// The [`AppView::key`], which [`super::DesktopCommand::ForceQuitApp`] quotes.
    pub app: String,
    pub standing: Standing,
}

/// The apps the panel lists, most recently used first: those with windows (a launching app has
/// none yet and a windowless one is a background helper, as the Mac omits both). An app does
/// not answer when its own life says so or any of its windows does not.
///
/// ```
/// use sill_model::desktop::{DesktopView, force_quit_rows};
/// assert!(force_quit_rows(&DesktopView::default()).is_empty());
/// ```
pub fn force_quit_rows(view: &DesktopView) -> Vec<ForceQuitRow> {
    let mut apps: Vec<&AppView> = view.apps.iter().filter(|app| listed(app)).collect();
    apps.sort_by_key(|app| app.mru);
    apps.into_iter()
        .map(|app| ForceQuitRow {
            app: app.key.clone(),
            standing: standing(view, app),
        })
        .collect()
}

fn listed(app: &AppView) -> bool {
    match app.life {
        AppLifeKind::Running | AppLifeKind::Quitting | AppLifeKind::NotResponding => true,
        AppLifeKind::Launching | AppLifeKind::Windowless => false,
    }
}

fn standing(view: &DesktopView, app: &AppView) -> Standing {
    let hung = app.life == AppLifeKind::NotResponding
        || view.windows.iter().any(|window| {
            window.app == app.key && window.responsiveness == Responsiveness::NotResponding
        });
    if hung {
        Standing::NotResponding
    } else {
        Standing::Responding
    }
}

/// The app the panel starts with selected: the first that does not answer, as the Mac selects a
/// hung app for you; none when every app answers.
pub fn hung_first(rows: &[ForceQuitRow]) -> Option<&str> {
    rows.iter()
        .find(|row| row.standing == Standing::NotResponding)
        .map(|row| row.app.as_str())
}

#[cfg(test)]
mod tests {
    use casement_protocols::vocab::{
        AppLifeKind, AppVisibility, Attention, Emphasis, LeaseState, Modality, PlacementMark,
        ProofKind, Responsiveness, SensitivityLevel, WindowRole, WindowStateKind,
    };

    use super::{ForceQuitRow, Standing, force_quit_rows, hung_first};
    use crate::desktop::{AppView, DesktopView, SpacePlaceView, WindowView};

    fn app(key: &str, life: AppLifeKind, mru: u32) -> AppView {
        AppView {
            key: key.to_owned(),
            proof: ProofKind::Scope,
            life,
            attention: Attention::None,
            mru,
            visibility: AppVisibility::Shown,
        }
    }

    fn window(app: &str, responsiveness: Responsiveness) -> WindowView {
        WindowView {
            identifier: format!("w-{app}"),
            app: app.to_owned(),
            title: String::new(),
            state: WindowStateKind::Floating,
            space: SpacePlaceView::Unknown,
            output: String::new(),
            emphasis: Emphasis::Inactive,
            placement: PlacementMark::Free,
            role: WindowRole::Document,
            modality: Modality::None,
            geometry: None,
            responsiveness,
            lease: LeaseState::Free,
            sensitivity: SensitivityLevel::Normal,
        }
    }

    fn row(app: &str, standing: Standing) -> ForceQuitRow {
        ForceQuitRow {
            app: app.to_owned(),
            standing,
        }
    }

    #[test]
    fn the_panel_lists_the_visible_apps_by_recency_and_marks_the_hung() {
        let view = DesktopView {
            apps: vec![
                app("files", AppLifeKind::Running, 2),
                app("mail", AppLifeKind::Running, 0),
                app("helper", AppLifeKind::Windowless, 1),
                app("starting", AppLifeKind::Launching, 3),
                app("term", AppLifeKind::NotResponding, 4),
                app("music", AppLifeKind::Quitting, 5),
                app("notes", AppLifeKind::Running, 6),
            ],
            windows: vec![
                window("files", Responsiveness::Responding),
                window("notes", Responsiveness::NotResponding),
            ],
            ..DesktopView::default()
        };
        assert_eq!(
            force_quit_rows(&view),
            vec![
                row("mail", Standing::Responding),
                row("files", Standing::Responding),
                row("term", Standing::NotResponding),
                row("music", Standing::Responding),
                row("notes", Standing::NotResponding),
            ]
        );
    }

    #[test]
    fn a_hung_app_starts_selected() {
        let cases: [(&str, Vec<ForceQuitRow>, Option<&str>); 3] = [
            ("nothing listed", vec![], None),
            (
                "everything answers",
                vec![row("mail", Standing::Responding)],
                None,
            ),
            (
                "the first hung one",
                vec![
                    row("mail", Standing::Responding),
                    row("term", Standing::NotResponding),
                    row("notes", Standing::NotResponding),
                ],
                Some("term"),
            ),
        ];
        for (name, rows, want) in cases {
            assert_eq!(hung_first(&rows), want, "{name}");
        }
    }
}
