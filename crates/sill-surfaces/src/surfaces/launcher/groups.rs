//! Which rows belong to this opening, and whether a group mixes providers. Pure.

use sill_launcher::{Generation, Item};
use sill_services::Results;

/// Whether a group holds one provider's rows or several providers'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mix {
    Single,
    Mixed,
}

/// The rows of `results` that belong to this opening: a generation later than the one the
/// launcher opened over. Anything older is a stale keystroke's, including the previous
/// opening's last query.
pub fn fresh(results: &Results, opened_over: Generation) -> &[Item] {
    if results.generation > opened_over {
        &results.items
    } else {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::fresh;
    use sill_launcher::{Generation, IconRef, Item, ItemId, ProviderKind, Score};
    use sill_services::Results;

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

    #[test]
    fn stale_generations_show_nothing() {
        let results = Results {
            generation: Generation(4),
            items: vec![
                item(ProviderKind::Apps, "Firefox", 950),
                item(ProviderKind::System, "Lock", 400),
            ],
            sections: Vec::new(),
        };
        assert!(fresh(&results, Generation(4)).is_empty(), "opened over it");
        assert_eq!(fresh(&results, Generation(3)).len(), 2);
    }
}
