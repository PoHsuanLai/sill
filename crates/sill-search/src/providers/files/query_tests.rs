//! `FilesProvider::query` against made-up indexes: what it asks, when it asks nothing, and what
//! it pushes. No test here reads a real index or the disk.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sill_launcher::{Cancel, Mime, Provider, ProviderKind, Query, Scope, UnixTime, VecSink};
use tokio::sync::watch;

use super::{
    FakeIndex, FileAnswer, FileAsk, FileHit, FileIndex, FilesProvider, HitLimit, IndexFault,
    IndexSource,
};
use sill_model::results::{FilesChoice, LauncherPolicy};

/// An index that records every ask and answers a fixed answer.
#[derive(Debug)]
struct Scripted {
    answer: FileAnswer,
    asks: Mutex<Vec<FileAsk>>,
}

impl FileIndex for Scripted {
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
        viewer_store: PathBuf::from("/nowhere/anyview"),
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
        .map(|items| items.iter().map(|i| i.title.clone()).collect())
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
    let subtitles: Vec<&str> = sink.0[0].iter().map(|i| i.subtitle.as_str()).collect();
    assert_eq!(subtitles, ["~/a"]);
}

/// A viewer store under `dir` holding `entries` (newest first), as the viewer writes it.
fn viewer_store(dir: &std::path::Path, entries: Vec<anyview_store::HistoryEntry>) -> PathBuf {
    let store = dir.join("anyview");
    std::fs::create_dir_all(&store).unwrap_or_else(|e| panic!("{e}"));
    let history = anyview_store::History { entries };
    let json = serde_json::to_string(&history).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(store.join("history.json"), json).unwrap_or_else(|e| panic!("{e}"));
    store
}

fn viewed(
    path: &std::path::Path,
    seconds_ago: u64,
    label: anyview_store::ResumeLabel,
) -> anyview_store::HistoryEntry {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    anyview_store::HistoryEntry {
        path: anyview_core::FilePath::new(path).unwrap_or_else(|e| panic!("{e}")),
        kind: anyview_core::FormatKind::Pdf,
        viewed: anyview_store::Viewed(now - seconds_ago),
        label,
    }
}

#[test]
fn a_file_the_viewer_remembers_is_a_row_the_index_did_not_bring() {
    let dir = sill_testkit::TempDir::new("files-viewed-row");
    let book = dir.path().join("elsewhere/novel.pdf");
    std::fs::create_dir_all(book.parent().unwrap_or(dir.path())).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(&book, b"%PDF").unwrap_or_else(|e| panic!("{e}"));
    let gone = dir.path().join("elsewhere/novel-gone.pdf");
    let store = viewer_store(
        dir.path(),
        vec![
            viewed(&book, 60, anyview_store::ResumeLabel::Page { number: 143 }),
            viewed(&gone, 30, anyview_store::ResumeLabel::Unlabelled),
        ],
    );
    let files = FilesProvider {
        viewer_store: store,
        home: dir.path().to_path_buf(),
        ..provider(FilesChoice::Auto, scripted(FileAnswer::Hits(Vec::new())))
    };
    let mut sink = VecSink::default();
    files.query(&Query::begin("novel", &Cancel::new()), &mut sink);
    assert_eq!(titles(&sink), [["novel.pdf"]], "the one still on disk");
    let row = &sink.0[0][0];
    assert_eq!(row.subtitle, "~/elsewhere · Page 143");
    assert!(
        row.actions.iter().any(|action| action.id.0 == "open"),
        "an ordinary file row, with the shared actions"
    );
}

#[test]
fn a_viewed_file_outranks_its_namesake_the_index_brought_with_the_same_age() {
    let dir = sill_testkit::TempDir::new("files-viewed-rank");
    let (a, b) = (
        dir.path().join("a/notes.pdf"),
        dir.path().join("b/notes.pdf"),
    );
    for file in [&a, &b] {
        std::fs::create_dir_all(file.parent().unwrap_or(dir.path()))
            .unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(file, b"%PDF").unwrap_or_else(|e| panic!("{e}"));
    }
    let index = Arc::new(FakeIndex {
        hits: vec![
            hit(a.to_str().unwrap_or_default()),
            hit(b.to_str().unwrap_or_default()),
        ],
    });
    let order = |entries: Vec<anyview_store::HistoryEntry>| {
        let files = FilesProvider {
            viewer_store: viewer_store(dir.path(), entries),
            home: dir.path().to_path_buf(),
            ..provider(FilesChoice::Auto, index.clone())
        };
        let mut sink = VecSink::default();
        files.query(&Query::begin("notes", &Cancel::new()), &mut sink);
        sink.0[0]
            .iter()
            .map(|row| row.id.0.clone())
            .collect::<Vec<String>>()
    };
    let (a_id, b_id) = (a.display().to_string(), b.display().to_string());
    let label = anyview_store::ResumeLabel::Unlabelled;
    assert_eq!(
        order(vec![viewed(&b, 10, label)]),
        [b_id.clone(), a_id.clone()],
        "b was viewed"
    );
    assert_eq!(
        order(vec![viewed(&a, 10, label)]),
        [a_id, b_id],
        "a was viewed"
    );
}
