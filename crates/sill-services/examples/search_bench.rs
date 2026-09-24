//! Query-to-first-batch latency of the launcher search, through the public
//! `sill_services::results::spawn` (M4 launcher-core, FINDINGS "M4 launcher-core").
//!
//! `cargo run -p sill-services --example search_bench --release`
//!
//! Two indexes: the fixture tree (`tests/fixtures/applications`, a dozen entries) and the same
//! entries copied to 1500 apps with distinct names (a large desktop). One stand-in apps
//! provider does the apps provider's work (`rank_text` over name, generic name and keywords),
//! so the numbers are the runner's cost plus realistic matching. Prints p50/p95/max over 200
//! queries after a warm-up.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sill_launcher::{
    Activation, AppId, Batch, Choice, FrecencyScore, IconRef, Item, ItemId, Needle, Provider,
    ProviderKind, Query, RankWeights, ResultSink, Score, matching::matcher, rank_text,
};
use sill_services::apps::{IconTheme, scan};
use sill_services::{AppIndex, AppsEvent, Listing, SearchCommand, reduce_apps, results};

const QUERIES: &[&str] = &[
    "fi", "firefox", "rich", "act", "only", "o", "fx", "ric", "ff", "fire", "app", "ac",
];
const RUNS: usize = 200;
const LARGE: usize = 1500;

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
                let score: Score = [&app.name, &app.generic_name]
                    .into_iter()
                    .chain(app.keywords.iter())
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

fn fixture() -> AppIndex {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/applications");
    let found = scan(
        &[root.join("home"), root.join("system")],
        &IconTheme("hicolor".to_owned()),
        &[],
    );
    reduce_apps(AppIndex::default(), AppsEvent::Scanned(found))
}

fn enlarged(base: &AppIndex, count: usize) -> AppIndex {
    let found = (0..count)
        .map(|n| {
            let mut app = base.apps[n % base.apps.len()].clone();
            app.id = AppId(format!("{}.copy{n}", app.id.0));
            app.name = format!("{} {n}", app.name);
            app.listing = Listing::Shown;
            app
        })
        .collect();
    reduce_apps(AppIndex::default(), AppsEvent::Scanned(found))
}

fn percentile(sorted: &[Duration], p: usize) -> Duration {
    sorted[(sorted.len() * p).div_ceil(100).max(1) - 1]
}

fn measure(rt: &tokio::runtime::Runtime, name: &str, index: AppIndex) {
    let apps = index.apps.len();
    let (mut rx, control) = results::spawn(rt.handle(), vec![Arc::new(IndexApps(index))]);
    let mut generation = 0u64;
    let mut ask = |text: &str| {
        generation += 1;
        let want = generation;
        let started = Instant::now();
        control
            .send(SearchCommand::Query(text.to_owned()))
            .unwrap_or_else(|e| panic!("{e}"));
        rt.block_on(rx.wait_for(|r| r.generation.0 == want && !r.items.is_empty()))
            .unwrap_or_else(|e| panic!("{e}"));
        started.elapsed()
    };
    for text in QUERIES {
        ask(text);
    }
    let mut samples: Vec<Duration> = (0..RUNS).map(|n| ask(QUERIES[n % QUERIES.len()])).collect();
    samples.sort();
    println!(
        "{name} ({apps} apps), {RUNS} queries: p50 {:?}, p95 {:?}, max {:?}",
        percentile(&samples, 50),
        percentile(&samples, 95),
        samples[RUNS - 1]
    );
}

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let base = fixture();
    let large = enlarged(&base, LARGE);
    measure(&rt, "fixture", base);
    measure(&rt, "large", large);
}
