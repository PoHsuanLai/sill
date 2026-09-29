//! Drives the launcher's files provider the way the runner does (its `query` on the blocking
//! pool), prints its rows, and optionally carries out the first row's activation through the
//! real launch service (`LaunchCommand::Open`): `dev/accept-launcher-files.sh` and
//! `tests/files_tracker_live.rs` run it in a scratch environment (a scratch `HOME`, a private
//! bus, a stub opener), never in the person's own.
//!
//! Usage: `files_probe --home DIR [--backend auto|tracker|fd|plocate|off] [--query TEXT]
//! [--budget-ms N] [--retry-ms N] [--choose primary|reveal|copy] [--settle-ms N] [--detect]`
//!
//! Prints `detect tracker=… fd=… plocate=…` for `--detect`, one `row\t<title>\t<folder>
//! \t<mime>` line per row, `rows <n>`, and for `--choose` what was done (`open <target>` or
//! `copy <text>`). `--retry-ms` asks again until a row comes back (an index still indexing).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sill_launcher::{
    ActionId, Activation, Cancel, Choice, Item, Provider, Query, Subject, VecSink,
};
use sill_services::apps::AppIndex;
use sill_services::launch::{self, LaunchCommand, OpenRequest};
use sill_services::providers::FilesProvider;
use sill_services::providers::files::tracker::TrackerIndex;
use sill_services::providers::files::walk::{WalkIndex, Walker};
use sill_services::providers::files::{
    FileAnswer, FileAsk, FileIndex, HitLimit, IndexSource, choice,
};
use sill_services::{FilesChoice, LauncherPolicy};
use tokio::sync::watch;

/// The token the probe hands the opener, so a stub can show it arrived.
const TOKEN: &str = "sill-accept-files-token";

#[derive(Debug)]
struct Args {
    home: PathBuf,
    backend: Backend,
    query: String,
    budget: Duration,
    retry: Duration,
    choose: Option<Choice>,
    settle: Duration,
    detect: Detect,
}

#[derive(Debug, Clone, Copy)]
enum Backend {
    Auto,
    Tracker,
    Fd,
    Plocate,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Detect {
    Ask,
    Skip,
}

fn main() {
    let args = parse(std::env::args().skip(1).collect());
    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    runtime.block_on(run(args));
}

async fn run(args: Args) {
    if args.detect == Detect::Ask {
        let found = tokio::task::spawn_blocking(choice::detect)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        println!(
            "detect tracker={:?} fd={:?} plocate={:?}",
            found.tracker, found.fd, found.plocate
        );
    }
    if args.query.is_empty() {
        return;
    }
    let provider = Arc::new(provider(&args));
    let rows = rows_within(&provider, &args).await;
    for row in &rows {
        let mime = match &row.subject {
            Subject::File(file) => file.mime.0.clone(),
            _ => String::new(),
        };
        println!("row\t{}\t{}\t{mime}", row.title, row.subtitle);
    }
    println!("rows {}", rows.len());
    if rows.is_empty() {
        println!("answer {}", raw_answer(&args).await);
    }
    if let Some(first) = rows.first() {
        choose(&provider, first, &args).await;
    }
}

/// What the chosen index itself answered, as its kind and a count (no paths): why no row came.
async fn raw_answer(args: &Args) -> String {
    let index: Arc<dyn FileIndex> = match args.backend {
        Backend::Tracker => Arc::new(TrackerIndex),
        Backend::Fd | Backend::Auto => Arc::new(WalkIndex { walker: Walker::Fd }),
        Backend::Plocate => Arc::new(WalkIndex {
            walker: Walker::Plocate,
        }),
        Backend::Off => return "off".to_owned(),
    };
    let ask = FileAsk {
        text: args.query.clone(),
        under: args.home.clone(),
        limit: HitLimit(40),
        budget: args.budget,
    };
    tokio::task::spawn_blocking(move || {
        match index.search(&ask, &Query::begin(ask.text.clone(), &Cancel::new())) {
            FileAnswer::Hits(hits) => format!("hits {}", hits.len()),
            FileAnswer::Partial(hits) => format!("partial {}", hits.len()),
            FileAnswer::Unavailable(fault) => format!("unavailable {fault:?}"),
        }
    })
    .await
    .unwrap_or_else(|e| panic!("{e}"))
}

fn provider(args: &Args) -> FilesProvider {
    let (files, source) = match args.backend {
        Backend::Auto => (FilesChoice::Auto, IndexSource::Detect),
        Backend::Tracker => (FilesChoice::Tracker, IndexSource::Detect),
        Backend::Fd => (FilesChoice::Walk, IndexSource::Detect),
        Backend::Plocate => (
            FilesChoice::Walk,
            IndexSource::Given(Arc::new(WalkIndex {
                walker: Walker::Plocate,
            })),
        ),
        Backend::Off => (FilesChoice::Off, IndexSource::Detect),
    };
    let policy = LauncherPolicy {
        files,
        files_budget: args.budget,
        ..LauncherPolicy::default()
    };
    FilesProvider {
        policy: watch::channel(policy).1,
        frecency: Arc::new(Mutex::default()),
        source,
        home: args.home.clone(),
    }
}

/// The provider's rows, asked again until some come back or `args.retry` passes.
async fn rows_within(provider: &Arc<FilesProvider>, args: &Args) -> Vec<Item> {
    let deadline = Instant::now() + args.retry;
    loop {
        let asked = Arc::clone(provider);
        let text = args.query.clone();
        let rows = tokio::task::spawn_blocking(move || {
            let mut sink = VecSink::default();
            asked.query(&Query::begin(text, &Cancel::new()), &mut sink);
            sink.0
                .into_iter()
                .flat_map(|batch| batch.items)
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_else(|e| panic!("{e}"));
        if !rows.is_empty() || Instant::now() >= deadline {
            return rows;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn choose(provider: &FilesProvider, row: &Item, args: &Args) {
    let Some(choice) = &args.choose else {
        return;
    };
    match provider.activate(row, choice) {
        Activation::Open { target } => {
            println!("open {target:?}");
            let (_apps_tx, apps) = watch::channel(AppIndex::default());
            let (_launches, control) = launch::spawn(&tokio::runtime::Handle::current(), apps);
            control
                .send(LaunchCommand::Open(OpenRequest {
                    target,
                    token: Some(TOKEN.to_owned()),
                }))
                .unwrap_or_else(|e| panic!("{e}"));
            tokio::time::sleep(args.settle).await;
        }
        Activation::Copy { text } => println!("copy {text}"),
        other => println!("other {other:?}"),
    }
}

fn parse(raw: Vec<String>) -> Args {
    let mut args = Args {
        home: PathBuf::new(),
        backend: Backend::Fd,
        query: String::new(),
        budget: Duration::from_millis(150),
        retry: Duration::ZERO,
        choose: None,
        settle: Duration::from_millis(1500),
        detect: Detect::Skip,
    };
    let mut rest = raw.into_iter();
    while let Some(flag) = rest.next() {
        if flag == "--detect" {
            args.detect = Detect::Ask;
            continue;
        }
        let value = rest
            .next()
            .unwrap_or_else(|| usage(&format!("{flag} needs a value")));
        let millis = || Duration::from_millis(value.parse().unwrap_or_else(|_| usage(&flag)));
        match flag.as_str() {
            "--home" => args.home = PathBuf::from(&value),
            "--backend" => args.backend = backend(&value),
            "--query" => args.query = value.clone(),
            "--budget-ms" => args.budget = millis(),
            "--retry-ms" => args.retry = millis(),
            "--choose" => args.choose = Some(choice_of(&value)),
            "--settle-ms" => args.settle = millis(),
            other => usage(&format!("unknown flag {other}")),
        }
    }
    if !args.query.is_empty() && !args.home.is_absolute() {
        usage("--home must be an absolute folder");
    }
    args
}

fn backend(name: &str) -> Backend {
    match name {
        "auto" => Backend::Auto,
        "tracker" => Backend::Tracker,
        "fd" => Backend::Fd,
        "plocate" => Backend::Plocate,
        "off" => Backend::Off,
        other => usage(&format!("unknown backend {other}")),
    }
}

fn choice_of(name: &str) -> Choice {
    match name {
        "primary" => Choice::Primary,
        "reveal" => Choice::Action(ActionId("reveal".to_owned())),
        "copy" => Choice::Action(ActionId("copy-path".to_owned())),
        other => usage(&format!("unknown choice {other}")),
    }
}

fn usage(problem: &str) -> ! {
    eprintln!("files_probe: {problem}");
    eprintln!(
        "usage: files_probe --home DIR [--backend auto|tracker|fd|plocate|off] \
         [--query TEXT] [--budget-ms N] [--retry-ms N] [--choose primary|reveal|copy] \
         [--settle-ms N] [--detect]"
    );
    std::process::exit(2);
}
