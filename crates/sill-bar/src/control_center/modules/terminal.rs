//! The Terminal module: what the terminal (`quire-do`) may do without asking, and the way back
//! when intentd's breaker paused it. Each action the person allowed from the confirmation sheet
//! ("Allow from the terminal: this action, until logout") is a row with its Revoke; a terminal
//! the breaker paused after too many refusals in a row shows why and a "Resume the terminal"
//! button (`Control.Resume`). The data is intentd's, read through the intents service; nothing is
//! drawn while there is nothing to say (no grant, no pause), and opening the control center reads
//! the grants again.

use dioxus::prelude::*;
use ds::components::content::label::{LabelRole, LabelStyle};
use ds::components::controls::button_model::Bezel;
use ds::prelude::*;
use ds::root::common::Common;
use sill_model::intents::terminal::{Breaker, Terminal, grant_words};
use sill_model::intents::{ActionRef, IntentsCommand};
use sill_services::IntentsService;
use sill_shell_kit::prelude::*;

use super::cc_module::{CcCx, CcModule, Span};
use crate::control_center::modules::use_plate;
use sill_ui_kit::hooks::{use_control, use_service_part};

/// One standing grant as a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GrantRow {
    pub action: String,
    pub app: String,
    pub grant: ActionRef,
}

/// What the module shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TerminalView {
    pub rows: Vec<GrantRow>,
    pub paused: Breaker,
}

/// The module's view, or `None` when there is nothing to say: no grant and no pause.
pub(crate) fn terminal_view(terminal: &Terminal) -> Option<TerminalView> {
    let rows: Vec<GrantRow> = terminal
        .grants
        .iter()
        .map(|grant| {
            let (action, app) = grant_words(grant);
            GrantRow {
                action,
                app,
                grant: grant.clone(),
            }
        })
        .collect();
    (!rows.is_empty() || terminal.breaker == Breaker::Tripped).then_some(TerminalView {
        rows,
        paused: terminal.breaker,
    })
}

/// The Terminal module.
#[component]
pub fn TerminalModule() -> Element {
    let terminal = use_service_part::<IntentsService, _>(|intents| intents.terminal.clone());
    let control = use_control::<IntentsService>();
    // The control center was opened: read the grants again (they change behind its back).
    let asked = control.clone();
    use_hook(move || {
        let _ = asked.send(IntentsCommand::RefreshGrants);
    });
    let Some(view) = terminal_view(&terminal.read()) else {
        return rsx! {};
    };
    let resume = control.clone();
    rsx! {
        ModulePanel { glyph: Icon::Terminal, title: "Terminal", plate: use_plate(),
            div { class: "sill-cc-terminal",
                if view.paused == Breaker::Tripped {
                    div { class: "sill-cc-terminal-paused",
                        Label {
                            text: "Paused: too many refusals in a row.",
                            role: LabelRole::Secondary,
                            style: LabelStyle::Footnote,
                        }
                        Button {
                            label: "Resume the terminal",
                            common: Common { id: Some("terminal-resume".to_owned()), ..Common::default() },
                            onclick: move |_| {
                                // A stopped service has nothing to resume.
                                let _gone = resume.send(IntentsCommand::ResumeTerminal);
                            },
                        }
                    }
                }
                if !view.rows.is_empty() {
                    Label {
                        text: "Allowed to run without asking, until you log out.",
                        role: LabelRole::Tertiary,
                        style: LabelStyle::Footnote,
                    }
                    div { class: "sill-cc-terminal-grants", role: "list",
                        for (place , row) in view.rows.iter().enumerate() {
                            {grant_row(place, row, &control)}
                        }
                    }
                }
            }
        }
    }
}

/// One grant: the action and its app, and its Revoke.
fn grant_row(
    place: usize,
    row: &GrantRow,
    control: &sill_services::Control<IntentsCommand>,
) -> Element {
    let (revoke, grant) = (control.clone(), row.grant.clone());
    rsx! {
        div { key: "{place}", class: "sill-cc-terminal-row", role: "listitem",
            div { class: "sill-cc-terminal-words",
                div { class: "sill-cc-terminal-action", "{row.action}" }
                div { class: "sill-cc-terminal-app", "in {row.app}" }
            }
            Button {
                bezel: Bezel::Inline,
                label: "Revoke",
                onclick: move |_| {
                    let _gone = revoke.send(IntentsCommand::RevokeGrant(grant.clone()));
                },
            }
        }
    }
}

/// The Terminal module as the control center draws it: a row, never a bar item, and absent while
/// it has nothing to say.
pub(crate) struct TerminalCc;

impl CcModule for TerminalCc {
    fn span(&self) -> Span {
        Span::Row
    }

    fn tile(&self, _cx: &CcCx) -> Element {
        rsx! { TerminalModule {} }
    }
}

#[cfg(test)]
mod tests {
    use sill_model::intents::ActionRef;
    use sill_model::intents::terminal::{Breaker, ControlReach, Terminal};

    use super::{GrantRow, TerminalView, terminal_view};

    fn grant(app: &str, name: &str) -> ActionRef {
        serde_json::from_str(&format!(r#"{{"app":"{app}","name":"{name}"}}"#))
            .unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn nothing_is_drawn_with_no_grant_and_no_pause() {
        let quiet = Terminal::default();
        assert_eq!(terminal_view(&quiet), None);
        let unreachable = Terminal {
            reach: ControlReach::Unreachable,
            ..Terminal::default()
        };
        assert_eq!(terminal_view(&unreachable), None);
    }

    #[test]
    fn grants_are_rows_and_a_pause_is_said_even_with_none() {
        let mail = grant("org.quire.Mail", "mail.thread.archive");
        let with = Terminal {
            grants: vec![mail.clone()],
            ..Terminal::default()
        };
        assert_eq!(
            terminal_view(&with),
            Some(TerminalView {
                rows: vec![GrantRow {
                    action: "Thread archive".to_owned(),
                    app: "Mail".to_owned(),
                    grant: mail,
                }],
                paused: Breaker::Running,
            })
        );
        let paused = Terminal {
            breaker: Breaker::Tripped,
            ..Terminal::default()
        };
        assert_eq!(
            terminal_view(&paused),
            Some(TerminalView {
                rows: Vec::new(),
                paused: Breaker::Tripped,
            })
        );
    }
}
