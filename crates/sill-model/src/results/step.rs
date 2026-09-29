//! The results' pure transition.

use super::{Results, ResultsEvent};

/// `results` after `event`. A batch from an older generation is dropped; one from a newer
/// generation (it overtook its own `Began`) starts that generation.
pub fn step(results: Results, event: ResultsEvent) -> Results {
    match event {
        ResultsEvent::Began(generation) if generation > results.generation => Results {
            generation,
            items: Vec::new(),
            sections: Vec::new(),
        },
        ResultsEvent::Began(_) => results,
        ResultsEvent::Batch(batch) if batch.generation < results.generation => results,
        ResultsEvent::Batch(batch) => {
            let mut items = if batch.generation > results.generation {
                Vec::new()
            } else {
                results.items
            };
            items.extend(batch.items);
            items.sort_by(|a, b| {
                b.score
                    .cmp(&a.score)
                    .then(a.provider.cmp(&b.provider))
                    .then_with(|| a.title.cmp(&b.title))
            });
            Results {
                generation: batch.generation,
                items,
                sections: Vec::new(),
            }
        }
    }
}
