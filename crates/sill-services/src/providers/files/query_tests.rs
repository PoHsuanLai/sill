//! `FilesProvider::query` against made-up indexes: what it asks, when it asks nothing, and what
//! it pushes. No test here reads a real index or the disk.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sill_launcher::{Cancel, Mime, Provider, ProviderKind, Query, Scope, UnixTime, VecSink};
use tokio::sync::watch;

use super::{
    FakeIndex, FileAnswer, FileAsk, FileHit, FileIndex, FilesProvider, HitLimit, IndexFault,
    IndexKind, IndexSource,
};
use sill_model::results::{FilesChoice, LauncherPolicy};

/// An index that records every ask and answers a fixed answer.
#[derive(Debug)]
struct Scripted {
    answer: FileAnswer,
    asks: Mutex<Vec<FileAsk>>,
}

impl FileIndex for Scripted {
    fn kind(&self) -> IndexKind {
        IndexKind::Fake
    }

    fn search(&self, ask: &FileAsk, _query: &Query) -> FileAnswer {
        self.asks
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(ask.clone());
        self.answer.clone()
    }
}

fn hit(path: &str) -> FileHit {
    let path = PathBuf::from(path);
    FileHit {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path,
        mime: Mime("application/pdf".to_owned()),
        modified: UnixTime(0),
    }
}

fn policy(files: FilesChoice) -> LauncherPolicy {
    LauncherPolicy {
        files,
        files_budget: Duration::from_millis(150),
        ..LauncherPolicy::default()
    }
}

fn provider(files: FilesChoice, index: Arc<dyn FileIndex>) -> FilesProvider {
    FilesProvider {
        policy: watch::channel(policy(files)).1,
        frecency: Arc::new(Mutex::default()),
        source: IndexSource::Given(index),
        home: PathBuf::from("/home/p"),
    }
}

fn scripted(answer: FileAnswer) -> Arc<Scripted> {
    Arc::new(Scripted {
        answer,
        asks: Mutex::new(Vec::new()),
    })
}

fn titles(sink: &VecSink) -> Vec<Vec<String>> {
    sink.0
        .iter()
        .map(|batch| batch.items.iter().map(|i| i.title.clone()).collect())
        .collect()
}

#[test]
fn one_ask_under_home_within_the_budget_and_its_rows_pushed() {
    let index = scripted(FileAnswer::Hits(vec![
        hit("/home/p/Documents/m9-report.pdf"),
        hit("/home/p/other.pdf"),
    ]));
    let files = provider(FilesChoice::Auto, index.clone());
    let mut sink = VecSink::default();
    let query = Query::begin("  m9-rep ", &Cancel::new());
    files.query(&query, &mut sink);
    let asks = index.asks.lock().map(|a| a.clone()).unwrap_or_default();
    assert_eq!(
        asks,
        [FileAsk {
            text: "m9-rep".to_owned(),
            under: PathBuf::from("/home/p"),
            limit: HitLimit(40),
            budget: Duration::from_millis(150),
        }]
    );
    assert_eq!(titles(&sink), [["m9-report.pdf"]]);
    assert_eq!(sink.0[0].provider, ProviderKind::Files);
    assert_eq!(sink.0[0].generation, query.generation);
}

#[test]
fn nothing_typed_or_off_asks_no_index() {
    let cases: &[(&str, FilesChoice, Scope)] = &[
        ("", FilesChoice::Auto, Scope::All),
        ("   ", FilesChoice::Walk, Scope::All),
        ("", FilesChoice::Auto, Scope::Only(ProviderKind::Files)),
        ("report", FilesChoice::Off, Scope::All),
    ];
    for &(text, choice, scope) in cases {
        let index = scripted(FileAnswer::Hits(vec![hit("/home/p/report.pdf")]));
        let mut sink = VecSink::default();
        let query = Query {
            scope,
            ..Query::begin(text, &Cancel::new())
        };
        provider(choice, index.clone()).query(&query, &mut sink);
        let asked = index.asks.lock().map(|a| a.len()).unwrap_or_default();
        assert_eq!(
            (asked, sink.0.len()),
            (0, 0),
            "{text:?} {choice:?} {scope:?}"
        );
    }
}

#[test]
fn a_partial_answer_is_pushed_and_an_unavailable_one_is_not() {
    let cases: Vec<(FileAnswer, Vec<Vec<&str>>)> = vec![
        (
            FileAnswer::Partial(vec![hit("/home/p/report.pdf")]),
            vec![vec!["report.pdf"]],
        ),
        (FileAnswer::Partial(Vec::new()), vec![vec![]]),
        (FileAnswer::Unavailable(IndexFault::NotRunning), vec![]),
    ];
    for (answer, want) in cases {
        let mut sink = VecSink::default();
        let label = format!("{answer:?}");
        provider(FilesChoice::Tracker, scripted(answer))
            .query(&Query::begin("report", &Cancel::new()), &mut sink);
        assert_eq!(titles(&sink), want, "{label}");
    }
}

#[test]
fn a_cancelled_query_pushes_nothing() {
    let cancel = Cancel::new();
    let stale = Query::begin("report", &cancel);
    let _newer = Query::begin("reports", &cancel);
    let index = Arc::new(FakeIndex {
        hits: vec![hit("/home/p/report.pdf")],
    });
    let mut sink = VecSink::default();
    provider(FilesChoice::Auto, index).query(&stale, &mut sink);
    assert!(sink.0.is_empty());
}

#[test]
fn the_fake_index_keeps_to_home() {
    let index = Arc::new(FakeIndex {
        hits: vec![hit("/home/p/a/report.pdf"), hit("/srv/report.pdf")],
    });
    let mut sink = VecSink::default();
    provider(FilesChoice::Auto, index).query(&Query::begin("report", &Cancel::new()), &mut sink);
    let subtitles: Vec<&str> = sink.0[0]
        .items
        .iter()
        .map(|i| i.subtitle.as_str())
        .collect();
    assert_eq!(subtitles, ["~/a"]);
}
