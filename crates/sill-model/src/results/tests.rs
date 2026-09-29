use sill_launcher::{Batch, Generation, IconRef, Item, ItemId, ProviderKind, Score};

use super::{Results, ResultsEvent, step};

fn item(provider: ProviderKind, title: &str, score: u32) -> Item {
    Item {
        id: ItemId(title.to_owned()),
        provider,
        title: title.to_owned(),
        subtitle: String::new(),
        icon: IconRef::None,
        score: Score(score),
        actions: Vec::new(),
        subject: sill_launcher::Subject::None,
    }
}

fn batch(generation: u64, provider: ProviderKind, items: &[(&str, u32)]) -> ResultsEvent {
    ResultsEvent::Batch(Batch {
        provider,
        generation: Generation(generation),
        items: items.iter().map(|&(t, s)| item(provider, t, s)).collect(),
    })
}

fn titles(r: &Results) -> (u64, Vec<&str>) {
    (
        r.generation.0,
        r.items.iter().map(|i| i.title.as_str()).collect(),
    )
}

fn after(events: Vec<ResultsEvent>) -> Results {
    events.into_iter().fold(Results::default(), step)
}

#[test]
fn batches_merge_best_first_and_stale_ones_drop() {
    use ProviderKind::{Apps, Calculator, System};
    type Case<'a> = (&'a str, Vec<ResultsEvent>, (u64, Vec<&'a str>));
    let cases: Vec<Case> = vec![
        (
            "two providers merge by score",
            vec![
                ResultsEvent::Began(Generation(1)),
                batch(1, Apps, &[("Files", 500), ("Firefox", 900)]),
                batch(1, System, &[("Restart", 700)]),
            ],
            (1, vec!["Firefox", "Restart", "Files"]),
        ),
        (
            "ties go by provider then title",
            vec![
                batch(1, System, &[("Lock", 500)]),
                batch(1, Apps, &[("b", 500), ("a", 500)]),
            ],
            (1, vec!["a", "b", "Lock"]),
        ),
        (
            "a new generation clears the old rows",
            vec![
                batch(1, Apps, &[("Files", 500)]),
                ResultsEvent::Began(Generation(2)),
            ],
            (2, vec![]),
        ),
        (
            "a stale batch is dropped",
            vec![
                ResultsEvent::Began(Generation(2)),
                batch(2, Apps, &[("Firefox", 900)]),
                batch(1, Calculator, &[("= 4", 1000)]),
            ],
            (2, vec!["Firefox"]),
        ),
        (
            "a batch that overtakes its Began starts its generation",
            vec![
                batch(1, Apps, &[("Files", 500)]),
                batch(3, Apps, &[("Foot", 400)]),
                ResultsEvent::Began(Generation(3)),
            ],
            (3, vec!["Foot"]),
        ),
        (
            "an older Began changes nothing",
            vec![
                batch(3, Apps, &[("Foot", 400)]),
                ResultsEvent::Began(Generation(2)),
            ],
            (3, vec!["Foot"]),
        ),
    ];
    for (name, events, want) in cases {
        assert_eq!(titles(&after(events)), want, "{name}");
    }
}
