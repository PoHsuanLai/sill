use super::super::*;
use super::{ask, candidates, output, window};

fn wire(reply: &Reply) -> Vec<(&str, &str)> {
    reply
        .sources
        .iter()
        .map(|(k, n)| (k.as_str(), n.as_str()))
        .collect()
}

#[test]
fn the_kinds_decide_the_tabs_and_what_each_lists() {
    let rows = [
        (1, vec![Tab::Screens], 2, 0),
        (2, vec![Tab::Windows], 0, 2),
        (3, vec![Tab::Screens, Tab::Windows], 2, 2),
    ];
    for (kinds, tabs, screens, windows) in rows {
        let plan = plan(&ask(kinds, 0, &[1]), &candidates());
        assert_eq!(plan.tabs, tabs, "kinds {kinds}");
        assert_eq!((plan.screens.len(), plan.windows.len()), (screens, windows));
    }
}

#[test]
fn one_source_replaces_and_many_add_and_remove() {
    let one = plan(&ask(3, 0, &[1]), &candidates());
    let picks = Picks::start(&one);
    let picks = toggle(&one, Plurality::One, picks, output("DP-1"));
    let picks = toggle(&one, Plurality::One, picks, window("w-2"));
    assert_eq!(picks.chosen, vec![window("w-2")]);
    let picks = toggle(&one, Plurality::One, picks, window("w-2"));
    assert_eq!(picks.chosen, vec![window("w-2")]);

    let many = plan(&ask(3, 1, &[1]), &candidates());
    let picks = Picks::start(&many);
    let picks = toggle(&many, Plurality::Many, picks, window("w-2"));
    let picks = toggle(&many, Plurality::Many, picks, output("DP-1"));
    assert_eq!(picks.chosen, vec![window("w-2"), output("DP-1")]);
    let picks = toggle(&many, Plurality::Many, picks, window("w-2"));
    assert_eq!(picks.chosen, vec![output("DP-1")]);
}

#[test]
fn a_source_that_is_not_offered_is_not_picked() {
    let screens_only = plan(&ask(1, 1, &[1]), &candidates());
    let picks = toggle(
        &screens_only,
        Plurality::Many,
        Picks::start(&screens_only),
        window("w-1"),
    );
    assert!(picks.chosen.is_empty());
    assert!(!can_allow(&screens_only, &picks));
}

#[test]
fn the_reply_lists_screens_then_windows_in_plan_order() {
    let many = plan(&ask(3, 1, &[1, 2]), &candidates());
    let mut picks = Picks::start(&many);
    for source in [
        window("w-2"),
        output("eDP-1"),
        window("w-1"),
        output("DP-1"),
    ] {
        picks = toggle(&many, Plurality::Many, picks, source);
    }
    picks.pointer = Pointer::Embedded;
    let got = reply(&many, &picks);
    assert_eq!(
        wire(&got),
        vec![
            ("output", "DP-1"),
            ("output", "eDP-1"),
            ("window", "w-1"),
            ("window", "w-2")
        ]
    );
    assert_eq!(got.cursor, 2);
}

#[test]
fn a_pointer_mode_the_ask_did_not_offer_falls_back_to_its_first() {
    let plan = plan(&ask(1, 0, &[2]), &candidates());
    let mut picks = toggle(&plan, Plurality::One, Picks::start(&plan), output("DP-1"));
    picks.pointer = Pointer::Metadata;
    assert_eq!(reply(&plan, &picks).cursor, 2);
}

#[test]
fn a_pick_that_has_gone_is_dropped_and_nothing_left_is_a_cancel() {
    let before = plan(&ask(2, 0, &[1]), &candidates());
    let picks = toggle(
        &before,
        Plurality::One,
        Picks::start(&before),
        window("w-1"),
    );
    let mut fewer = candidates();
    fewer.windows.remove(0);
    let after = super::super::plan(&ask(2, 0, &[1]), &fewer);
    assert_eq!(reply(&after, &picks), Reply::cancelled());
    assert!(!can_allow(&after, &picks));
}
