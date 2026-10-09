//! How a timeline row names a coding agent: its label, else its provider row, else its program.

use almanac_core::{Count, EntryBody, EventRef, KindTag, ReplicaId, Seq, TimelineEntry, UndoRef};
use porter_core::ProviderId;
use porter_core::sheet::{ProviderKind, ProviderRow, RowKind};
use prov::{Actor, AgentLabel, AgentProgram, ClientName};

use super::{ProgramLabels, memory_line};

fn entry(actor: Actor) -> TimelineEntry {
    TimelineEntry {
        event: EventRef {
            space: prov::SpaceId::desktop(),
            replica: ReplicaId([2; 16]),
            seq: Seq(1),
        },
        occurred: prov::UnixSeconds(1),
        actor,
        kind: KindTag::parse("thing.archived").unwrap_or_else(|e| panic!("{e}")),
        effect: prov::Effect::UndoableWrite,
        label: prov::Label::trusted_user(),
        things: Vec::new(),
        body: EntryBody::Present,
        derived_facts: Count(0),
        undo: UndoRef::None,
    }
}

fn acp(program: &str, label: Option<&str>) -> Actor {
    Actor::Acp {
        program: AgentProgram::parse(program).unwrap_or_else(|e| panic!("{e}")),
        label: label.map(|text| AgentLabel(text.to_owned())),
    }
}

fn rows() -> ProgramLabels {
    let row = |id: &str, label: &str, auth| ProviderRow {
        id: ProviderId::parse(id).unwrap_or_else(|e| panic!("{e}")),
        label: label.to_owned(),
        mark: String::new(),
        kind: RowKind::Provider,
        auth,
        mark_face: None,
        group: None,
    };
    ProgramLabels::of_rows(&[
        row("claude-code", "Claude Code", ProviderKind::AgentLogin),
        row("nextcloud", "Nextcloud", ProviderKind::Service),
    ])
}

fn who(actor: Actor, programs: &ProgramLabels) -> String {
    memory_line(&entry(actor), programs).0
}

#[test]
fn the_persons_label_comes_first() {
    assert_eq!(
        who(acp("claude-code", Some("My Claude")), &rows()),
        "My Claude (assistant) archived"
    );
}

#[test]
fn the_provider_rows_label_comes_second() {
    assert_eq!(
        who(acp("claude-code", None), &rows()),
        "Claude Code (assistant) archived"
    );
}

#[test]
fn an_unnamed_program_is_never_shown_as_written() {
    assert_eq!(who(acp("codex", None), &rows()), "An assistant archived");
    assert_eq!(
        who(acp("claude-code", None), &ProgramLabels::default()),
        "An assistant archived"
    );
}

#[test]
fn a_label_is_never_taken_from_anywhere_else() {
    // A service row is not a program, and an Mcp client's own name is shown as it is, not
    // swapped for a row's label.
    assert_eq!(
        who(acp("nextcloud", None), &rows()),
        "An assistant archived"
    );
    let client = ClientName::parse("claude-code").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        who(Actor::Mcp { client }, &rows()),
        "An outside assistant archived"
    );
}
