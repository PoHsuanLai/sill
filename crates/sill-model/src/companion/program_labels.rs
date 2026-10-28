//! What a coding agent's program is called, where the person can read it: the label of the
//! provider row porter lists for it (`claude-code` is "Claude Code").

use std::collections::BTreeMap;

use porter_sheet::{ProviderKind, ProviderRow};
use prov::AgentLabel;

/// Provider-row labels by program id. Only agent-login rows name a program.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgramLabels(BTreeMap<String, String>);

impl ProgramLabels {
    /// The labels of the agent-login rows among `rows`.
    pub fn of_rows(rows: &[ProviderRow]) -> ProgramLabels {
        ProgramLabels(
            rows.iter()
                .filter(|row| row.auth == ProviderKind::AgentLogin)
                .map(|row| (row.id.as_str().to_owned(), row.label.clone()))
                .collect(),
        )
    }

    /// How a coding agent reads: the name the person gave it in agents.toml (the actor's own
    /// label), else the provider row's label for its program; `None` when neither names it (the
    /// program id is a developer word and is never shown). The agent's own handshake title is
    /// never read.
    pub fn name_of(&self, program: &str, label: Option<&AgentLabel>) -> Option<String> {
        label
            .map(|label| label.0.as_str())
            .or_else(|| self.0.get(program).map(String::as_str))
            .map(str::to_owned)
    }
}
