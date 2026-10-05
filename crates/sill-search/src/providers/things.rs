//! The things apps index (mail, contacts, events), found through intentd's shadow index
//! (`Intents1.Search`). The companion service holds the shell's seat at intentd, so a lookup is a
//! command to it and its answer a state it publishes: `query` sends the ask under the query's
//! generation and waits for the answer to that generation, or for a newer ask to replace it.
//!
//! A row is the thing as its app named it; Enter performs the app's `<kind>.open` action on it as
//! the launcher, with no session, when the app's manifest declares that action. A thing whose app
//! declares none has no Enter action. Hits an app answers late arrive as more rows of the same
//! generation while the query waits.

use docket_core::{ActionRef, CallRequest, Hit, Origin, TargetValue, open_name};
use sill_launcher::{
    Activation, Batch, Choice, IconRef, Item, ItemId, Provider, ProviderKind, Query, ResultSink,
    Score, Subject,
};
use sill_model::companion::{Companion, CompanionCommand, CompanionLink, LookupPhase, Opener};
use sill_services::control::Control;
use tokio::sync::watch;

use super::lookup::block_on;

/// Rows from the apps' indexes.
#[derive(Debug, Clone)]
pub struct ThingsProvider {
    /// The companion service's state, where the answer lands.
    pub companion: watch::Receiver<Companion>,
    /// Where the ask goes.
    pub control: Control<CompanionCommand>,
}

impl Provider for ThingsProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Things
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        let text = query.text.trim();
        let ready = matches!(self.companion.borrow().link, CompanionLink::Ready);
        let hits = if text.is_empty() || !ready || query.is_cancelled() {
            Vec::new()
        } else {
            self.found(text, query)
        };
        if query.is_cancelled() {
            return;
        }
        sink.push(Batch {
            provider: ProviderKind::Things,
            generation: query.generation,
            items: hits
                .iter()
                .enumerate()
                .map(|(at, hit)| row(hit, at))
                .collect(),
        });
    }

    fn activate(&self, item: &Item, choice: &Choice) -> Activation {
        match (&item.subject, choice) {
            (Subject::Entity(entity), Choice::Primary) => {
                let opener = Opener {
                    app: entity.id.app.clone(),
                    kind: entity.id.kind.clone(),
                };
                if !self.companion.borrow().openers.contains(&opener) {
                    return Activation::Nothing;
                }
                open_call(&entity.id).map_or(Activation::Nothing, |call| {
                    Activation::Intent(Box::new(call))
                })
            }
            _ => Activation::Nothing,
        }
    }
}

impl ThingsProvider {
    /// The hits of one ask, once the service has answered it: the indexed ones, and any an app
    /// sent late before a newer ask replaced this one.
    fn found(&self, text: &str, query: &Query) -> Vec<Hit> {
        let mut state = self.companion.clone();
        let generation = query.generation;
        if self
            .control
            .send(CompanionCommand::Things {
                text: text.to_owned(),
                generation,
            })
            .is_err()
        {
            return Vec::new();
        }
        block_on(async move {
            let answered = state
                .wait_for(|companion| {
                    let things = &companion.things;
                    things.generation > generation
                        || (things.generation == generation
                            && matches!(things.phase, LookupPhase::Answered | LookupPhase::Failed))
                })
                .await
                .ok()?;
            let things = &answered.things;
            (things.generation == generation).then(|| things.found.clone())
        })
        .flatten()
        .unwrap_or_default()
    }
}

/// One hit as a row. The order the apps gave is kept: the score falls by position.
fn row(hit: &Hit, at: usize) -> Item {
    let entity = &hit.entity;
    let subtitle = match (&hit.why, entity.subtitle.value.as_str()) {
        (Some(why), "") => why.as_str().to_owned(),
        (_, subtitle) => subtitle.to_owned(),
    };
    Item {
        id: ItemId(format!(
            "{}:{}:{}",
            entity.id.app.as_str(),
            entity.id.kind.as_str(),
            entity.id.key.as_str()
        )),
        provider: ProviderKind::Things,
        title: entity.title.value.clone(),
        subtitle,
        icon: IconRef::Glyph("inbox".to_owned()),
        score: Score(u32::try_from(1000usize.saturating_sub(at)).unwrap_or(0)),
        actions: Vec::new(),
        subject: Subject::Entity(Box::new(entity.clone())),
    }
}

/// The call that opens `thing` in its app: the kind's `open` action on the thing itself.
fn open_call(thing: &prov::EntityId) -> Option<CallRequest> {
    let name = open_name(&thing.kind)?;
    Some(CallRequest {
        action: ActionRef {
            app: thing.app.clone(),
            name,
        },
        target: TargetValue::Entities(vec![thing.clone()]),
        args: docket_core::Args::new(),
        origin: Origin::Launcher,
    })
}

#[cfg(test)]
mod tests {
    use prov::{AppName, EntityKey, EntityKind, Label, Labelled};
    use sill_launcher::{Cancel, Generation, VecSink};
    use sill_model::companion::Lookup;
    use sill_services::control::Control;

    use super::*;

    fn hit(key: &str, title: &str) -> Hit {
        Hit {
            entity: docket_core::EntityRef {
                id: prov::EntityId {
                    app: AppName::parse("org.quire.Mail").unwrap_or_else(|e| panic!("{e}")),
                    kind: EntityKind::parse("mail.thread").unwrap_or_else(|e| panic!("{e}")),
                    key: EntityKey::parse(key).unwrap_or_else(|e| panic!("{e}")),
                },
                title: Labelled::new(title.to_owned(), Label::trusted_user()),
                subtitle: Labelled::new("Sam".to_owned(), Label::trusted_user()),
            },
            why: None,
        }
    }

    /// The one kind the stand-in app opens: its threads.
    fn mail_opens_threads() -> std::collections::BTreeSet<Opener> {
        let thread = &hit("a", "x").entity.id;
        std::iter::once(Opener {
            app: thread.app.clone(),
            kind: thread.kind.clone(),
        })
        .collect()
    }

    /// The companion service as the provider sees it: told a lookup, it answers with `found`
    /// for the generation it was asked.
    fn service(found: Vec<Hit>) -> ThingsProvider {
        let (state_tx, state_rx) = watch::channel(Companion {
            link: CompanionLink::Ready,
            openers: mail_opens_threads(),
            ..Companion::default()
        });
        let (control, mut commands) = Control::channel();
        tokio::spawn(async move {
            while let Some(command) = commands.recv().await {
                if let CompanionCommand::Things { generation, .. } = command {
                    state_tx.send_modify(|c| {
                        c.things = Lookup {
                            generation,
                            phase: LookupPhase::Answered,
                            found: found.clone(),
                        };
                    });
                }
            }
        });
        ThingsProvider {
            companion: state_rx,
            control,
        }
    }

    async fn rows(provider: ThingsProvider, text: &'static str) -> Vec<Item> {
        tokio::task::spawn_blocking(move || {
            let mut sink = VecSink::default();
            provider.query(&Query::begin(text, &Cancel::new()), &mut sink);
            sink.0.into_iter().flat_map(|batch| batch.items).collect()
        })
        .await
        .unwrap_or_else(|e| panic!("{e}"))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_hits_of_the_ask_become_rows_in_the_order_the_apps_gave() {
        let provider = service(vec![
            hit("a", "Lisbon receipts"),
            hit("b", "Lisbon flights"),
        ]);
        let found = rows(provider, "lisbon").await;
        let shown: Vec<(&str, &str)> = found
            .iter()
            .map(|i| (i.title.as_str(), i.subtitle.as_str()))
            .collect();
        assert_eq!(
            shown,
            [("Lisbon receipts", "Sam"), ("Lisbon flights", "Sam")]
        );
        assert!(
            found[0].score > found[1].score,
            "the apps' order is the rank"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn nothing_typed_asks_nothing() {
        let (state_tx, state_rx) = watch::channel(Companion {
            link: CompanionLink::Ready,
            ..Companion::default()
        });
        let (control, mut commands) = Control::channel();
        let provider = ThingsProvider {
            companion: state_rx,
            control,
        };
        assert!(rows(provider, "  ").await.is_empty());
        assert!(commands.try_recv().is_err(), "no ask went out");
        drop(state_tx);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn enter_opens_the_thing_with_its_apps_open_action() {
        let provider = service(Vec::new());
        let row = row(&hit("a", "Lisbon receipts"), 0);
        match provider.activate(&row, &Choice::Primary) {
            Activation::Intent(call) => {
                assert_eq!(call.action.name.as_str(), "mail.thread.open");
                assert_eq!(call.action.app.as_str(), "org.quire.Mail");
                assert_eq!(call.origin, Origin::Launcher);
                assert!(
                    matches!(call.target, TargetValue::Entities(ref things) if things.len() == 1)
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_thing_whose_app_declares_no_open_action_has_no_enter_action() {
        let (_state_tx, state_rx) = watch::channel(Companion {
            link: CompanionLink::Ready,
            ..Companion::default()
        });
        let (control, _commands) = Control::channel();
        let provider = ThingsProvider {
            companion: state_rx,
            control,
        };
        let row = row(&hit("a", "Lisbon receipts"), 0);
        assert_eq!(
            provider.activate(&row, &Choice::Primary),
            Activation::Nothing
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_newer_ask_replaces_the_one_a_provider_waits_on() {
        let (state_tx, state_rx) = watch::channel(Companion {
            link: CompanionLink::Ready,
            ..Companion::default()
        });
        let (control, mut commands) = Control::channel();
        let provider = ThingsProvider {
            companion: state_rx,
            control,
        };
        let waiting = tokio::spawn(rows(provider, "lis"));
        // The provider asked; then a newer keystroke's ask lands before any answer.
        let CompanionCommand::Things { generation, .. } =
            commands.recv().await.unwrap_or_else(|| panic!("an ask"))
        else {
            panic!("a lookup");
        };
        state_tx.send_modify(|c| {
            c.things = Lookup {
                generation: Generation(generation.0 + 1),
                phase: LookupPhase::Asked,
                found: Vec::new(),
            };
        });
        assert!(waiting.await.unwrap_or_else(|e| panic!("{e}")).is_empty());
    }
}
