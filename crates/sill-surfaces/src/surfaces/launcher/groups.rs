//! The results as the palette shows them (design/06 §11.3's grouping, read for the launcher):
//! an empty query lists Actions (the system provider's rows) then Recent (the five best of the
//! rest); a query lists the Top hit, then each provider's rows in Tab order, at most six each;
//! a Tab filter lists that provider's rows alone. Pure.

use sill_launcher::{Generation, Item, ProviderKind};
use sill_services::Results;

use crate::surfaces::launcher::keys::Filter;

/// Rows per provider group under a query (design/06 §11.3 caps mail at six).
pub const GROUP_CAP: usize = 6;

/// Rows under Recent for the empty query (design/06 §11.3: "top 5").
pub const RECENT_CAP: usize = 5;

/// One group's heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupTitle {
    TopHit,
    Actions,
    Recent,
    Provider(ProviderKind),
}

impl GroupTitle {
    /// The heading's words.
    pub fn text(self) -> &'static str {
        match self {
            GroupTitle::TopHit => "Top hit",
            GroupTitle::Actions => "Actions",
            GroupTitle::Recent => "Recent",
            GroupTitle::Provider(kind) => provider_name(kind),
        }
    }

    /// Whether the group mixes providers, so a row's kind is worth a note.
    pub fn mixes(self) -> Mix {
        match self {
            GroupTitle::TopHit | GroupTitle::Recent => Mix::Mixed,
            GroupTitle::Actions | GroupTitle::Provider(_) => Mix::Single,
        }
    }
}

/// Whether a group holds one provider's rows or several providers'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mix {
    Single,
    Mixed,
}

/// A provider's group heading.
pub fn provider_name(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Apps => "Applications",
        ProviderKind::Windows => "Windows",
        ProviderKind::Calculator => "Calculator",
        ProviderKind::Settings => "Settings",
        ProviderKind::System => "System",
    }
}

/// One group: its heading and rows, best first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub title: GroupTitle,
    pub rows: Vec<Item>,
}

/// The rows of `results` that belong to this opening: a generation later than the one the
/// launcher opened over. Anything older is a stale keystroke's (plan "stale generations
/// dropped"), including the previous opening's last query.
pub fn fresh(results: &Results, opened_over: Generation) -> &[Item] {
    if results.generation > opened_over {
        &results.items
    } else {
        &[]
    }
}

/// `items` (best first, as `Results` keeps them) grouped for `query` under `filter`.
pub fn groups(items: &[Item], query: &str, filter: Filter) -> Vec<Group> {
    let groups = match filter {
        Filter::Only(kind) => vec![Group {
            title: GroupTitle::Provider(kind),
            rows: of(items, kind).cloned().collect(),
        }],
        Filter::All if query.trim().is_empty() => empty_query(items),
        Filter::All => searched(items),
    };
    groups.into_iter().filter(|g| !g.rows.is_empty()).collect()
}

/// Every row, in the order the palette lists them (headers not counted).
pub fn rows(groups: &[Group]) -> Vec<&Item> {
    groups.iter().flat_map(|g| g.rows.iter()).collect()
}

fn of(items: &[Item], kind: ProviderKind) -> impl Iterator<Item = &Item> {
    items.iter().filter(move |item| item.provider == kind)
}

fn empty_query(items: &[Item]) -> Vec<Group> {
    vec![
        Group {
            title: GroupTitle::Actions,
            rows: of(items, ProviderKind::System).cloned().collect(),
        },
        Group {
            title: GroupTitle::Recent,
            rows: items
                .iter()
                .filter(|item| item.provider != ProviderKind::System)
                .take(RECENT_CAP)
                .cloned()
                .collect(),
        },
    ]
}

fn searched(items: &[Item]) -> Vec<Group> {
    let Some((top, rest)) = items.split_first() else {
        return Vec::new();
    };
    std::iter::once(Group {
        title: GroupTitle::TopHit,
        rows: vec![top.clone()],
    })
    .chain(ProviderKind::ALL.iter().map(|kind| Group {
        title: GroupTitle::Provider(*kind),
        rows: of(rest, *kind).take(GROUP_CAP).cloned().collect(),
    }))
    .collect()
}

/// The next filter from `filter` that shows a row, stepping with `next`; `Filter::All` when
/// none does (Tab skips providers that answered nothing).
pub fn next_shown(
    items: &[Item],
    query: &str,
    filter: Filter,
    next: impl Fn(Filter) -> Filter,
) -> Filter {
    let mut candidate = next(filter);
    for _ in 0..=ProviderKind::ALL.len() {
        if candidate == Filter::All || !groups(items, query, candidate).is_empty() {
            return candidate;
        }
        candidate = next(candidate);
    }
    Filter::All
}

#[cfg(test)]
mod tests {
    use super::{GroupTitle, fresh, groups, next_shown, rows};
    use crate::surfaces::launcher::keys::{Filter, next_filter, previous_filter};
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
        }
    }

    fn sample() -> Vec<Item> {
        use ProviderKind::{Apps, Calculator, Settings, System, Windows};
        vec![
            item(Apps, "Firefox", 950),
            item(Windows, "Firefox — Mail", 900),
            item(Apps, "Files", 700),
            item(System, "Log Out", 650),
            item(Settings, "Display", 600),
            item(Calculator, "= 4", 500),
            item(System, "Lock", 400),
            item(Apps, "Fonts", 300),
        ]
    }

    fn shape(query: &str, filter: Filter) -> Vec<(&'static str, Vec<String>)> {
        groups(&sample(), query, filter)
            .into_iter()
            .map(|g| {
                (
                    g.title.text(),
                    g.rows.into_iter().map(|i| i.title).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn a_query_lists_the_top_hit_then_providers_in_tab_order() {
        assert_eq!(
            shape("f", Filter::All),
            vec![
                ("Top hit", vec!["Firefox".to_owned()]),
                ("Applications", vec!["Files".to_owned(), "Fonts".to_owned()]),
                ("Windows", vec!["Firefox — Mail".to_owned()]),
                ("Calculator", vec!["= 4".to_owned()]),
                ("Settings", vec!["Display".to_owned()]),
                ("System", vec!["Log Out".to_owned(), "Lock".to_owned()]),
            ]
        );
    }

    #[test]
    fn the_empty_query_lists_actions_then_recent() {
        let got = shape("  ", Filter::All);
        assert_eq!(
            got[0],
            ("Actions", vec!["Log Out".to_owned(), "Lock".to_owned()])
        );
        assert_eq!(got[1].0, "Recent");
        assert_eq!(
            got[1].1,
            vec!["Firefox", "Firefox — Mail", "Files", "Display", "= 4"],
            "five, system rows excluded"
        );
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn a_filter_lists_one_provider_whole_and_empty_groups_go() {
        assert_eq!(
            shape("f", Filter::Only(ProviderKind::Apps)),
            vec![(
                "Applications",
                vec!["Firefox".to_owned(), "Files".to_owned(), "Fonts".to_owned()]
            )]
        );
        let lone = vec![item(ProviderKind::Apps, "Files", 1)];
        let got = groups(&lone, "f", Filter::All);
        assert_eq!(
            got.len(),
            1,
            "the top hit alone; the Applications group is empty"
        );
        assert_eq!(got[0].title, GroupTitle::TopHit);
        assert!(groups(&[], "f", Filter::All).is_empty(), "nothing matches");
        assert_eq!(rows(&groups(&sample(), "f", Filter::All)).len(), 8);
    }

    #[test]
    fn a_group_caps_at_six() {
        let many: Vec<Item> = (0..10)
            .map(|i| item(ProviderKind::Apps, &format!("App {i}"), 100 - i))
            .collect();
        let got = groups(&many, "app", Filter::All);
        assert_eq!(got[1].rows.len(), 6);
        assert_eq!(
            groups(&many, "app", Filter::Only(ProviderKind::Apps))[0]
                .rows
                .len(),
            10
        );
    }

    #[test]
    fn stale_generations_show_nothing() {
        let results = Results {
            generation: Generation(4),
            items: sample(),
        };
        assert!(fresh(&results, Generation(4)).is_empty(), "opened over it");
        assert_eq!(fresh(&results, Generation(3)).len(), 8);
    }

    #[test]
    fn tab_skips_providers_with_nothing() {
        let only_apps_and_system = vec![
            item(ProviderKind::Apps, "Files", 2),
            item(ProviderKind::System, "Lock", 1),
        ];
        let tab = |f| next_shown(&only_apps_and_system, "l", f, next_filter);
        assert_eq!(tab(Filter::All), Filter::Only(ProviderKind::Apps));
        assert_eq!(
            tab(Filter::Only(ProviderKind::Apps)),
            Filter::Only(ProviderKind::System)
        );
        assert_eq!(tab(Filter::Only(ProviderKind::System)), Filter::All);
        let back = |f| next_shown(&only_apps_and_system, "l", f, previous_filter);
        assert_eq!(back(Filter::All), Filter::Only(ProviderKind::System));
        assert_eq!(next_shown(&[], "x", Filter::All, next_filter), Filter::All);
    }
}
