//! The published order of the rows: the command menu's groups (design/06 §20.2 "the command
//! menu's fuzzy score and grouping (section 11)", §11.3 "Result groups"), mapped onto the
//! launcher's providers. Pure.
//!
//! `Results` is one flat list (frozen), so the groups are an ordering the list view reads back
//! by rule (FINDINGS F102):
//!
//! - typed text: row 0 is the Top hit (the single best row of every provider); after it, one
//!   run per provider in Tab order ([`ProviderKind::ALL`]), each capped by [`group_cap`] and
//!   best first. The Top hit is not repeated in its provider's run.
//! - nothing typed: the System rows (the launcher's Actions, all of them), then Recent: the
//!   [`RECENT_CAP`] other rows chosen most frecently, best first. A row never chosen is not
//!   shown.

use sill_launcher::{Frecency, FrecencyScore, Item, ItemKey, Needle, ProviderKind, UnixTime};

/// How many rows Recent shows when nothing is typed (§11.3: "Recent (top 5 mail)").
pub(crate) const RECENT_CAP: usize = 5;

/// Whether anything was typed. Whitespace alone is nothing (the same rule as
/// [`Needle::is_empty`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Typed {
    Nothing,
    Text,
}

impl Typed {
    /// What `text` counts as.
    pub(crate) fn of(text: &str) -> Typed {
        if Needle::parse(text).is_empty() {
            Typed::Nothing
        } else {
            Typed::Text
        }
    }
}

/// The most rows one provider's run shows under typed text, after the Top hit. §11.3 caps
/// Mail at 6 and People and Actions at 3: apps take Mail's 6 (the launcher's main content),
/// the calculator has one answer, the rest take 3.
pub(crate) fn group_cap(kind: ProviderKind) -> usize {
    match kind {
        ProviderKind::Apps => 6,
        ProviderKind::Calculator => 1,
        ProviderKind::Windows | ProviderKind::Settings | ProviderKind::System => 3,
    }
}

/// Under typed text: `merged` (best first, as `reduce_results` keeps it) as the Top hit, then
/// each provider's run in Tab order.
pub(crate) fn grouped(merged: &[Item]) -> Vec<Item> {
    let Some((top, rest)) = merged.split_first() else {
        return Vec::new();
    };
    let runs = ProviderKind::ALL.into_iter().flat_map(|kind| {
        rest.iter()
            .filter(move |item| item.provider == kind)
            .take(group_cap(kind))
    });
    std::iter::once(top).chain(runs).cloned().collect()
}

/// With nothing typed: every System row of `merged`, then its most frecent other rows at
/// `now`.
pub(crate) fn actions_then_recent(
    merged: &[Item],
    frecency: &Frecency,
    now: UnixTime,
) -> Vec<Item> {
    let actions = merged
        .iter()
        .filter(|item| item.provider == ProviderKind::System);
    let mut recent: Vec<(FrecencyScore, &Item)> = merged
        .iter()
        .filter(|item| item.provider != ProviderKind::System)
        .map(|item| {
            (
                frecency.score(&ItemKey::of(item.provider, &item.id), now),
                item,
            )
        })
        .filter(|(score, _)| *score > FrecencyScore::NONE)
        .collect();
    // Stable: equal frecency keeps `merged`'s order.
    recent.sort_by_key(|&(score, _)| std::cmp::Reverse(score));
    actions
        .chain(recent.into_iter().take(RECENT_CAP).map(|(_, item)| item))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{RECENT_CAP, Typed, actions_then_recent, group_cap, grouped};
    use sill_launcher::{Frecency, IconRef, Item, ItemId, ItemKey, ProviderKind, Score, UnixTime};

    use ProviderKind::{Apps, Calculator, Settings, System, Windows};

    const NOW: UnixTime = UnixTime(1_790_000_000);

    fn item(provider: ProviderKind, title: &str, score: u32) -> Item {
        Item {
            id: ItemId(title.to_owned()),
            provider,
            title: title.to_owned(),
            subtitle: String::new(),
            icon: IconRef::None,
            score: Score(score),
            actions: Vec::new(),
        }
    }

    fn titles(items: &[Item]) -> Vec<&str> {
        items.iter().map(|i| i.title.as_str()).collect()
    }

    /// `merged`, sorted as `reduce_results` keeps it.
    fn merged(rows: &[(ProviderKind, &str, u32)]) -> Vec<Item> {
        let mut items: Vec<Item> = rows.iter().map(|&(p, t, s)| item(p, t, s)).collect();
        items.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(a.provider.cmp(&b.provider))
                .then_with(|| a.title.cmp(&b.title))
        });
        items
    }

    #[test]
    fn typing_is_classified_like_the_needle() {
        const CASES: &[(&str, Typed)] = &[
            ("", Typed::Nothing),
            ("   ", Typed::Nothing),
            ("f", Typed::Text),
            (" fi ", Typed::Text),
            ("2+2", Typed::Text),
        ];
        for &(text, want) in CASES {
            assert_eq!(Typed::of(text), want, "{text:?}");
        }
    }

    #[test]
    fn typed_text_is_top_hit_then_providers_in_tab_order() {
        type Case<'a> = (&'a str, Vec<(ProviderKind, &'a str, u32)>, Vec<&'a str>);
        let cases: Vec<Case> = vec![
            ("nothing matched", vec![], vec![]),
            (
                "one row is its own top hit",
                vec![(System, "Lock", 500)],
                vec!["Lock"],
            ),
            (
                "the best row leads, then Tab order, not score order",
                vec![
                    (System, "Restart", 900),
                    (Apps, "Files", 400),
                    (Settings, "Resolution", 800),
                    (Apps, "Firefox", 700),
                    (Windows, "foot ~", 300),
                ],
                vec!["Restart", "Firefox", "Files", "foot ~", "Resolution"],
            ),
            (
                "the top hit is not repeated in its own run",
                vec![
                    (Apps, "Files", 900),
                    (Apps, "Firefox", 700),
                    (Calculator, "= 4", 600),
                ],
                vec!["Files", "Firefox", "= 4"],
            ),
        ];
        for (name, rows, want) in cases {
            let out = grouped(&merged(&rows));
            assert_eq!(titles(&out), want, "{name}");
        }
    }

    #[test]
    fn every_run_is_capped_after_the_top_hit() {
        let rows: Vec<(ProviderKind, String, u32)> = ProviderKind::ALL
            .into_iter()
            .flat_map(|p| (0..10u32).map(move |n| (p, format!("{p:?}{n}"), 100 + n)))
            .collect();
        let borrowed: Vec<_> = rows.iter().map(|(p, t, s)| (*p, t.as_str(), *s)).collect();
        let out = grouped(&merged(&borrowed));
        // Every provider's best scores 109; a tie goes by Tab order, so Apps's leads.
        assert_eq!(out[0].title, "Apps9");
        for kind in ProviderKind::ALL {
            let shown = out[1..].iter().filter(|i| i.provider == kind).count();
            assert_eq!(
                shown,
                group_cap(kind).min(10 - usize::from(kind == Apps)),
                "{kind:?}"
            );
        }
        let providers: Vec<ProviderKind> = out[1..].iter().map(|i| i.provider).collect();
        let mut sorted = providers.clone();
        sorted.sort();
        assert_eq!(providers, sorted, "runs are in Tab order");
    }

    #[test]
    fn nothing_typed_is_every_action_then_the_most_frecent_rows() {
        let rows = merged(&[
            (System, "Lock", 0),
            (System, "Suspend", 0),
            (Apps, "Files", 30),
            (Apps, "Firefox", 90),
            (Apps, "Never used", 0),
            (Windows, "foot ~", 0),
            (Settings, "Wallpaper", 0),
        ]);
        let key = |p, id: &str| ItemKey::of(p, &ItemId(id.to_owned()));
        let frecency = Frecency::default()
            .record(key(Apps, "Files"), NOW)
            .record(key(Apps, "Firefox"), NOW)
            .record(key(Apps, "Firefox"), NOW)
            .record(key(Windows, "foot ~"), NOW)
            .record(key(Windows, "foot ~"), NOW)
            .record(key(Windows, "foot ~"), NOW)
            .record(key(Settings, "Wallpaper"), UnixTime(NOW.0 - 86_400 * 200))
            .record(key(System, "Lock"), NOW);
        let out = actions_then_recent(&rows, &frecency, NOW);
        assert_eq!(
            titles(&out),
            ["Lock", "Suspend", "foot ~", "Firefox", "Files", "Wallpaper"]
        );
    }

    #[test]
    fn recent_stops_at_its_cap() {
        let names: Vec<String> = (0..9).map(|n| format!("app{n}")).collect();
        let rows = merged(
            &names
                .iter()
                .map(|n| (Apps, n.as_str(), 0))
                .collect::<Vec<_>>(),
        );
        let frecency = names.iter().fold(Frecency::default(), |f, n| {
            f.record(ItemKey::of(Apps, &ItemId(n.clone())), NOW)
        });
        let out = actions_then_recent(&rows, &frecency, NOW);
        assert_eq!(out.len(), RECENT_CAP);
    }
}
