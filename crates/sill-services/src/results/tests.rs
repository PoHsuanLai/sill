//! The runner end to end on a real runtime, with fake providers: ordering and grouping,
//! generations dropped however late they arrive, cancellation, choices counted and saved, and
//! the query-to-first-batch latency over the fixture app index.

mod fake;
mod latency;

use std::sync::Arc;
use std::time::Duration;

use sill_launcher::{Choice, ItemId, ItemKey, Provider, ProviderKind};
use tokio::runtime::Handle;
use tokio::sync::watch;

use super::history::History;
use super::test_support::TempDir;
use super::{Results, SearchCommand, load_frecency, start};
use crate::control::Control;
use fake::{Fake, Heard, Obeys};

/// How long a test waits for a published value before it fails.
const PATIENCE: Duration = Duration::from_secs(5);

fn titles(results: &Results) -> Vec<&str> {
    results.items.iter().map(|i| i.title.as_str()).collect()
}

/// The first published value satisfying `ready`.
async fn until(rx: &mut watch::Receiver<Results>, ready: impl FnMut(&Results) -> bool) -> Results {
    let found = tokio::time::timeout(PATIENCE, rx.wait_for(ready))
        .await
        .map(|seen| seen.map(|value| value.clone()));
    match found {
        Ok(Ok(value)) => value,
        Ok(Err(e)) => panic!("{e}"),
        Err(_) => panic!("timed out; last {:?}", titles(&rx.borrow())),
    }
}

fn search(
    providers: Vec<Arc<dyn Provider>>,
    dir: &TempDir,
) -> (watch::Receiver<Results>, Control<SearchCommand>, History) {
    let history = History::default();
    let (rx, control) = start(
        &Handle::current(),
        providers,
        history.clone(),
        Some(dir.path().to_path_buf()),
    );
    (rx, control, history)
}

fn send(control: &Control<SearchCommand>, command: SearchCommand) {
    control.send(command).unwrap_or_else(|e| panic!("{e}"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_first_batch_shows_before_the_second_and_both_merge_in_groups() {
    let dir = TempDir::new("runner-order");
    let apps = Fake::new(ProviderKind::Apps)
        .first(&[("Firefox", 700), ("Files", 400)])
        .then(Duration::from_millis(150), &[("Fire Starter", 900)]);
    let system = Fake::new(ProviderKind::System).first(&[("Restart", 500)]);
    let (mut rx, control, _) = search(vec![apps.provider(), system.provider()], &dir);

    send(&control, SearchCommand::Query("fi".to_owned()));
    let early = until(&mut rx, |r| r.items.len() == 3).await;
    assert_eq!(
        titles(&early),
        ["Firefox", "Files", "Restart"],
        "top hit, apps, system"
    );
    let late = until(&mut rx, |r| r.items.len() == 4).await;
    assert_eq!(
        titles(&late),
        ["Fire Starter@fi", "Firefox", "Files", "Restart"],
        "the late best row becomes the top hit"
    );
    assert_eq!(early.generation, late.generation);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_newer_keystroke_cancels_the_older_query_and_its_late_rows_never_show() {
    let dir = TempDir::new("runner-cancel");
    let obeys = Fake::new(ProviderKind::Apps)
        .first(&[("Firefox", 700)])
        .then(Duration::from_millis(150), &[("Late", 999)]);
    // A provider that ignores cancellation and pushes its late rows anyway.
    let deaf = Fake::new(ProviderKind::Settings)
        .obeys(Obeys::Never)
        .then(Duration::from_millis(150), &[("Deaf", 998)]);
    let (mut rx, control, _) = search(vec![obeys.provider(), deaf.provider()], &dir);

    send(&control, SearchCommand::Query("f".to_owned()));
    let first = until(&mut rx, |r| !r.items.is_empty()).await;
    obeys.clear_first();
    send(&control, SearchCommand::Query("fo".to_owned()));
    let second = until(&mut rx, |r| r.generation > first.generation).await;
    // Both halves of both generations have run (the first's while the second is current).
    tokio::time::timeout(PATIENCE, async {
        while obeys.heard().len() < 2 || deaf.heard().len() < 2 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("heard {:?} and {:?}", obeys.heard(), deaf.heard()));
    // Let the last batch cross the channel and the runner.
    until(&mut rx, |r| r.items.len() == 2).await;

    let shown = rx.borrow().clone();
    assert_eq!(shown.generation, second.generation);
    assert_eq!(
        titles(&shown),
        ["Late@fo", "Deaf@fo"],
        "only the current generation's rows; `Late@f` and `Deaf@f` were dropped"
    );
    assert_eq!(
        obeys.heard(),
        [Heard::Cancelled, Heard::Finished],
        "the overtaken query stopped early; the newer one ran to the end"
    );
    assert_eq!(deaf.heard(), [Heard::Finished, Heard::Finished]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closing_cancels_the_running_query_and_empties_the_list() {
    let dir = TempDir::new("runner-close");
    let slow = Fake::new(ProviderKind::Apps)
        .first(&[("Firefox", 700)])
        .then(Duration::from_millis(150), &[("Late", 999)]);
    let (mut rx, control, _) = search(vec![slow.provider()], &dir);

    send(&control, SearchCommand::Query("f".to_owned()));
    let first = until(&mut rx, |r| !r.items.is_empty()).await;
    send(&control, SearchCommand::Cancel);
    let closed = until(&mut rx, |r| r.generation > first.generation).await;
    assert!(closed.items.is_empty());
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(*rx.borrow(), closed, "the late batch never lands");
    assert_eq!(slow.heard(), [Heard::Cancelled]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_query_nothing_matches_publishes_an_empty_list_once_every_provider_is_done() {
    let dir = TempDir::new("runner-empty");
    let apps = Fake::new(ProviderKind::Apps).first(&[("Firefox", 700)]);
    let (mut rx, control, _) = search(vec![apps.provider()], &dir);
    send(&control, SearchCommand::Query("f".to_owned()));
    let first = until(&mut rx, |r| !r.items.is_empty()).await;
    apps.clear_first();
    send(&control, SearchCommand::Query("zzz".to_owned()));
    let none = until(&mut rx, |r| r.generation > first.generation).await;
    assert!(none.items.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_panicking_provider_does_not_hold_up_the_others() {
    let dir = TempDir::new("runner-panic");
    let apps = Fake::new(ProviderKind::Apps).first(&[("Firefox", 700)]);
    let broken = Fake::new(ProviderKind::Calculator).panics();
    let (mut rx, control, _) = search(vec![broken.provider(), apps.provider()], &dir);
    send(&control, SearchCommand::Query("f".to_owned()));
    assert_eq!(
        titles(&until(&mut rx, |r| !r.items.is_empty()).await),
        ["Firefox"]
    );
    apps.clear_first();
    send(&control, SearchCommand::Query("g".to_owned()));
    let next = until(&mut rx, |r| r.items.is_empty()).await;
    assert!(
        next.generation.0 >= 2,
        "the empty answer was published, not stuck pending"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_choice_is_counted_saved_and_shown_under_recent() {
    let dir = TempDir::new("runner-chosen");
    let apps = Fake::new(ProviderKind::Apps).first(&[("Firefox", 0), ("Files", 0)]);
    let system = Fake::new(ProviderKind::System).first(&[("Lock", 0)]);
    let (mut rx, control, history) = search(vec![apps.provider(), system.provider()], &dir);

    let files = ItemKey::of(ProviderKind::Apps, &ItemId("Files".to_owned()));
    send(
        &control,
        SearchCommand::Chosen {
            key: files.clone(),
            choice: Choice::Primary,
        },
    );
    send(&control, SearchCommand::Query(String::new()));
    let shown = until(&mut rx, |r| r.items.len() == 2).await;
    assert_eq!(titles(&shown), ["Lock", "Files"], "Actions, then Recent");
    assert_eq!(
        history.read(|f| f.entries.get(&files).map(|u| u.count)),
        Some(1)
    );

    let saved = tokio::time::timeout(PATIENCE, async {
        loop {
            let on_disk = load_frecency(dir.path());
            if on_disk.entries.contains_key(&files) {
                return on_disk;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("frecency.json never written"));
    assert_eq!(saved, history.read(Clone::clone));
}

#[tokio::test]
async fn the_runner_itself_drops_a_batch_of_an_older_generation() {
    use super::history::Persist;
    use super::run::Search;
    use super::sink::Arrival;
    use sill_launcher::{Batch, Generation, IconRef, Item, Score};

    let row = |title: &str| Item {
        id: ItemId(title.to_owned()),
        provider: ProviderKind::Apps,
        title: title.to_owned(),
        subtitle: String::new(),
        icon: IconRef::None,
        score: Score(500),
        actions: Vec::new(),
    };
    let batch = |generation, title| {
        Arrival::Batch(Batch {
            provider: ProviderKind::Apps,
            generation: Generation(generation),
            items: vec![row(title)],
        })
    };
    let (tx, rx) = watch::channel(Results::default());
    let (mut search, _arrivals) = Search::new(Vec::new(), History::default(), Persist::Nowhere, tx);
    search.command(SearchCommand::Query("f".to_owned()));
    search.command(SearchCommand::Query("fi".to_owned()));
    search.arrive(batch(1, "late"));
    assert_eq!(rx.borrow().generation, Generation(2));
    assert!(rx.borrow().items.is_empty(), "{:?}", titles(&rx.borrow()));
    search.arrive(batch(2, "current"));
    assert_eq!(titles(&rx.borrow()), ["current"]);
    search.arrive(batch(1, "later still"));
    assert_eq!(titles(&rx.borrow()), ["current"]);
}
