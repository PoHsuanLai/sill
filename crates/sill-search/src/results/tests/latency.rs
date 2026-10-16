//! Query to first batch over the fixture app index (`sill-services/tests/fixtures/applications`), through the
//! whole runner: `Control::send`, a new generation, a blocking task per provider, the batch on
//! the channel, `step`, grouping and the watch. Brief: p95 under 10 ms over 200
//! queries on a warm index. `sill-testkit`'s `examples/search_bench.rs` measures a larger index in
//! release.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sill_launcher::matching::matcher;
use sill_launcher::{
    Activation, Batch, Choice, FrecencyScore, IconRef, Item, ItemId, Needle, Provider,
    ProviderKind, Query, RankWeights, ResultSink, Score, rank_text,
};
use sill_model::apps::{AppIndex, AppsEvent, Listing};
use sill_model::results::SearchCommand;

use super::{TempDir, search, send};
use sill_services::apps::{IconRoots, IconTheme, scan};

/// Every query matches at least one fixture app.
const QUERIES: &[&str] = &[
    "fi", "firefox", "rich", "act", "only", "o", "fx", "ric", "ff", "fire", "app", "ac",
];
const RUNS: usize = 200;
const BUDGET: Duration = Duration::from_millis(10);

/// A stand-in for the apps provider (providers-a fills the real one in parallel): the same
/// work, `rank_text` over name, generic name and keywords of every listed app, one batch.
struct IndexApps(AppIndex);

impl Provider for IndexApps {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Apps
    }

    fn query(&self, query: &Query, sink: &mut dyn ResultSink) {
        let needle = Needle::parse(&query.text);
        let mut m = matcher();
        let w = RankWeights::DEFAULT;
        let items = self
            .0
            .apps
            .iter()
            .filter(|app| app.listing == Listing::Shown)
            .filter_map(|app| {
                let fields = [&app.name, &app.generic_name]
                    .into_iter()
                    .chain(app.keywords.iter());
                let score: Score = fields
                    .filter_map(|text| rank_text(&needle, text, FrecencyScore::NONE, &w, &mut m))
                    .max()?;
                Some(Item {
                    id: ItemId(app.id.0.clone()),
                    provider: ProviderKind::Apps,
                    title: app.name.clone(),
                    subtitle: app.comment.clone(),
                    icon: IconRef::None,
                    score,
                    actions: Vec::new(),
                    subject: sill_launcher::Subject::None,
                })
            })
            .collect();
        sink.push(Batch {
            provider: ProviderKind::Apps,
            generation: query.generation,
            items,
        });
    }

    fn activate(&self, _item: &Item, _choice: &Choice) -> Activation {
        Activation::Nothing
    }
}

fn fixture_index() -> AppIndex {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../sill-services/tests/fixtures/applications");
    let found = scan(
        &[root.join("home"), root.join("system")],
        &IconTheme("hicolor".to_owned()),
        &IconRoots::default(),
        &[],
    );
    sill_model::apps::step(AppIndex::default(), AppsEvent::Scanned(found))
}

/// The `p`th percentile (nearest rank) of sorted `samples`.
fn percentile(samples: &[Duration], p: usize) -> Duration {
    let rank = (samples.len() * p).div_ceil(100).max(1);
    samples[rank - 1]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_to_first_batch_p95_is_under_10_ms_on_the_fixture_index() {
    let dir = TempDir::new("runner-latency");
    let index = fixture_index();
    assert!(
        index.apps.len() >= 10,
        "fixture scanned: {}",
        index.apps.len()
    );
    let (mut rx, control, _) = search(vec![Arc::new(IndexApps(index))], &dir);

    let mut generation = 0u64;
    let mut ask = async |text: &str| {
        generation += 1;
        let want = generation;
        let started = Instant::now();
        send(&control, SearchCommand::Query(text.to_owned()));
        tokio::time::timeout(
            Duration::from_secs(5),
            rx.wait_for(|r| r.generation.0 == want && !r.items.is_empty()),
        )
        .await
        .unwrap_or_else(|_| panic!("{text:?} never answered"))
        .unwrap_or_else(|e| panic!("{e}"));
        started.elapsed()
    };

    // Warm: the blocking pool's threads exist and the index is in cache.
    for text in QUERIES {
        ask(text).await;
    }
    let mut samples = Vec::with_capacity(RUNS);
    for n in 0..RUNS {
        samples.push(ask(QUERIES[n % QUERIES.len()]).await);
    }
    samples.sort();
    let (p50, p95, max) = (
        percentile(&samples, 50),
        percentile(&samples, 95),
        samples[RUNS - 1],
    );
    eprintln!("query to first batch over {RUNS} queries: p50 {p50:?}, p95 {p95:?}, max {max:?}");
    assert!(p95 < BUDGET, "p95 {p95:?} (p50 {p50:?}, max {max:?})");
}
