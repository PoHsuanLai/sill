//! What the companion remembers, as the memory timeline: newest first, who did what to which thing.
//! Only the Memory filter (`Scope::Only`) lists it: the timeline is read from memoryd on demand,
//! through the companion service (which holds the shell's seat), and waited for under the
//! query's generation as the things provider waits for a search.

use almanac_core::{MemoryItem, TimelineEntry};
use sill_launcher::{
    Activation, Batch, Choice, IconRef, Item, ItemId, Provider, ProviderKind, Query, ResultSink,
    Scope, Score, Subject,
};
use sill_model::companion::{Companion, CompanionCommand, CompanionLink, LookupPhase, memory_line};
use sill_services::control::Control;
use tokio::sync::watch;

use super::lookup::block_on;

/// Rows from memoryd's timeline.
#[derive(Debug, Clone)]
pub struct MemoryProvider {
    pub companion: watch::Receiver<Companion>,
    pub control: Control<CompanionCommand>,
}

impl Provider for MemoryProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Memory
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        let alone = query.scope == Scope::Only(ProviderKind::Memory);
        let ready = matches!(self.companion.borrow().link, CompanionLink::Ready);
        let entries = if alone && ready && !query.is_cancelled() {
            self.timeline(query)
        } else {
            Vec::new()
        };
        if query.is_cancelled() {
            return;
        }
        let needle = query.text.trim().to_lowercase();
        sink.push(Batch {
            provider: ProviderKind::Memory,
            generation: query.generation,
            items: entries
                .iter()
                .enumerate()
                .map(|(at, entry)| row(entry, at))
                .filter(|item| needle.is_empty() || item.title.to_lowercase().contains(&needle))
                .collect(),
        });
    }

    fn activate(&self, _item: &Item, _choice: &Choice) -> Activation {
        Activation::Nothing
    }
}

impl MemoryProvider {
    /// The timeline's newest page, once the service has read it for this query.
    fn timeline(&self, query: &Query) -> Vec<TimelineEntry> {
        let mut state = self.companion.clone();
        let generation = query.generation;
        if self
            .control
            .send(CompanionCommand::Timeline { generation })
            .is_err()
        {
            return Vec::new();
        }
        block_on(async move {
            let answered = state
                .wait_for(|companion| {
                    let memory = &companion.memory;
                    memory.generation > generation
                        || (memory.generation == generation
                            && matches!(memory.phase, LookupPhase::Answered | LookupPhase::Failed))
                })
                .await
                .ok()?;
            let memory = &answered.memory;
            (memory.generation == generation).then(|| memory.found.clone())
        })
        .flatten()
        .unwrap_or_default()
    }
}

fn row(entry: &TimelineEntry, at: usize) -> Item {
    let (title, subtitle) = memory_line(entry);
    Item {
        id: ItemId(format!(
            "memory:{}:{:?}:{}",
            entry.event.space.as_str(),
            entry.event.replica,
            entry.event.seq.0
        )),
        provider: ProviderKind::Memory,
        title,
        subtitle,
        icon: IconRef::Glyph("clock".to_owned()),
        score: Score(u32::try_from(1000usize.saturating_sub(at)).unwrap_or(0)),
        actions: Vec::new(),
        subject: Subject::Memory(MemoryItem::Event(entry.event.clone())),
    }
}

#[cfg(test)]
mod tests {
    use almanac_core::Count;
    use almanac_core::{
        EntryBody, EventRef, KindTag, ReplicaId, Seq, ThingView, UndoRef, UserText,
    };
    use sill_launcher::{Cancel, VecSink};
    use sill_model::companion::Lookup;
    use sill_services::control::Control;

    use super::*;

    fn entry(seq: u64, title: &str) -> TimelineEntry {
        TimelineEntry {
            event: EventRef {
                space: prov::SpaceId::desktop(),
                replica: ReplicaId([2; 16]),
                seq: Seq(seq),
            },
            occurred: prov::UnixSeconds(1),
            actor: prov::Actor::Cli,
            kind: KindTag::parse("thing.archived").unwrap_or_else(|e| panic!("{e}")),
            effect: prov::Effect::UndoableWrite,
            label: prov::Label::trusted_user(),
            things: vec![ThingView {
                thing: prov::EntityId {
                    app: prov::AppName::parse("org.quire.Mail").unwrap_or_else(|e| panic!("{e}")),
                    kind: prov::EntityKind::parse("mail.thread").unwrap_or_else(|e| panic!("{e}")),
                    key: prov::EntityKey::parse("q4").unwrap_or_else(|e| panic!("{e}")),
                },
                title: UserText::new(title),
                subtitle: UserText::new(""),
            }],
            body: EntryBody::Present,
            derived_facts: Count(0),
            undo: UndoRef::None,
        }
    }

    fn provider(rows: Vec<TimelineEntry>) -> MemoryProvider {
        let (state_tx, state_rx) = watch::channel(Companion {
            link: CompanionLink::Ready,
            ..Companion::default()
        });
        let (control, mut commands) = Control::channel();
        tokio::spawn(async move {
            while let Some(command) = commands.recv().await {
                if let CompanionCommand::Timeline { generation } = command {
                    state_tx.send_modify(|c| {
                        c.memory = Lookup {
                            generation,
                            phase: LookupPhase::Answered,
                            found: rows.clone(),
                        };
                    });
                }
            }
        });
        MemoryProvider {
            companion: state_rx,
            control,
        }
    }

    async fn rows(provider: MemoryProvider, text: &'static str, scope: Scope) -> Vec<Item> {
        tokio::task::spawn_blocking(move || {
            let mut sink = VecSink::default();
            let query = Query {
                scope,
                ..Query::begin(text, &Cancel::new())
            };
            provider.query(&query, &mut sink);
            sink.0.into_iter().flat_map(|batch| batch.items).collect()
        })
        .await
        .unwrap_or_else(|e| panic!("{e}"))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn only_the_memory_filter_reads_the_timeline() {
        let found = rows(
            provider(vec![entry(1, "Q4 budget")]),
            "",
            Scope::Only(ProviderKind::Memory),
        )
        .await;
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].title,
            "Cli archived Q4 budget".replace("Cli", "A terminal")
        );
        assert!(
            rows(provider(vec![entry(1, "Q4 budget")]), "", Scope::All)
                .await
                .is_empty(),
            "a search never reads the timeline"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn typed_text_narrows_the_timeline_to_the_rows_that_say_it() {
        let both = provider(vec![entry(1, "Q4 budget"), entry(2, "Lisbon receipts")]);
        let found = rows(both, "lisbon", Scope::Only(ProviderKind::Memory)).await;
        assert_eq!(found.len(), 1);
        assert!(found[0].title.contains("Lisbon receipts"));
    }
}
