//! The events service end to end over a temp directory (never a real calendar): a file and a
//! vdir collection are read once told, and an edit of the file is read again within a second.
//! `#[ignore]`d because it watches the file system with real timers: run with
//! `cargo test -p sill-services --test events_live -- --ignored`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use jiff::Zoned;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use sill_services::env::Dirs;
use sill_services::events::{self, Agenda, EventSources, EventsCommand, IcsPlace, SourceStatus};
use tokio::sync::watch;

/// A temp directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("sill-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("dav")).unwrap_or_else(|e| panic!("{e}"));
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn today() -> Date {
    let zone = TimeZone::try_system().unwrap_or(TimeZone::UTC);
    Zoned::now().with_time_zone(zone).date()
}

/// One all-day event on `day`.
fn calendar(uid: &str, title: &str, day: Date) -> String {
    format!(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:{uid}\r\nSUMMARY:{title}\r\n\
         DTSTART;VALUE=DATE:{}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        day.strftime("%Y%m%d")
    )
}

/// Write through a temp file and a rename, as editors and vdirsyncer do.
fn save(path: &Path, text: &str) {
    let part = path.with_extension("part");
    std::fs::write(&part, text).unwrap_or_else(|e| panic!("{e}"));
    std::fs::rename(&part, path).unwrap_or_else(|e| panic!("{e}"));
}

fn titles(agenda: &Agenda) -> Vec<String> {
    let mut titles: Vec<_> = agenda.events.iter().map(|e| e.title.clone()).collect();
    titles.sort();
    titles
}

/// Wait up to `limit` for `done` to hold of the agenda.
async fn wait_for(
    rx: &mut watch::Receiver<Agenda>,
    limit: Duration,
    done: impl Fn(&Agenda) -> bool,
) -> Result<Agenda, Agenda> {
    let until = tokio::time::Instant::now() + limit;
    loop {
        if done(&rx.borrow()) {
            return Ok(rx.borrow().clone());
        }
        match tokio::time::timeout_at(until, rx.changed()).await {
            Ok(Ok(())) => {}
            Ok(Err(_)) | Err(_) => return Err(rx.borrow().clone()),
        }
    }
}

#[test]
#[ignore = "watches a temp directory with real timers"]
fn a_file_and_a_collection_are_read_and_an_edit_is_read_again_within_a_second() {
    let scratch = Scratch::new("events-live");
    let file = scratch.0.join("personal.ics");
    let day = today();
    save(&file, &calendar("a", "Lunch", day));
    save(&scratch.0.join("dav/b.ics"), &calendar("b", "Review", day));

    let runtime = tokio::runtime::Runtime::new().unwrap_or_else(|e| panic!("{e}"));
    let (mut rx, control) = events::spawn(runtime.handle(), &Dirs::under(&scratch.0));
    runtime.block_on(async {
        let passive = wait_for(&mut rx, Duration::from_millis(300), |a| {
            a.status != SourceStatus::Off
        })
        .await;
        assert!(passive.is_err(), "passive until told: {passive:?}");

        let sources = EventSources::Places(vec![
            IcsPlace::File(file.clone()),
            IcsPlace::Dir(scratch.0.join("dav")),
        ]);
        control
            .send(EventsCommand::Sources(sources))
            .unwrap_or_else(|e| panic!("{e}"));
        let read = wait_for(&mut rx, Duration::from_secs(2), |a| a.events.len() == 2).await;
        let read = read.unwrap_or_else(|a| panic!("not read: {a:?}"));
        assert_eq!(titles(&read), ["Lunch", "Review"]);
        assert_eq!(
            read.status,
            SourceStatus::Read {
                calendars: 2,
                problems: Vec::new()
            }
        );

        // Let the watch settle before the edit, then time the edit alone.
        tokio::time::sleep(Duration::from_millis(200)).await;
        let edited = std::time::Instant::now();
        save(&file, &calendar("a", "Lunch with Mei", day));
        let again = wait_for(&mut rx, Duration::from_secs(1), |a| {
            titles(a).contains(&"Lunch with Mei".to_owned())
        })
        .await;
        let again = again.unwrap_or_else(|a| panic!("the edit was not read in 1 s: {a:?}"));
        println!("edit read again after {:?}", edited.elapsed());
        assert_eq!(titles(&again), ["Lunch with Mei", "Review"]);

        control
            .send(EventsCommand::Sources(EventSources::Off))
            .unwrap_or_else(|e| panic!("{e}"));
        let off = wait_for(&mut rx, Duration::from_secs(1), |a| {
            a.status == SourceStatus::Off
        })
        .await;
        assert!(off.is_ok_and(|a| a.events.is_empty()), "switched off");
    });
}
