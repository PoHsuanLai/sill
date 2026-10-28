use super::needing;
use crate::accounts::{AccountKey, AccountsEffect as Fx, AccountsEvent as Ev, AccountsState, step};

fn known(keys: &[&str]) -> AccountsState {
    AccountsState {
        present: true,
        needing: keys.iter().map(|key| needing(key, key)).collect(),
    }
}

fn keys(state: &AccountsState) -> Vec<&str> {
    state.needing.iter().map(|n| n.account.as_str()).collect()
}

/// One row: what was known, the event, the accounts needing sign-in after it, the effects.
struct Case {
    name: &'static str,
    before: AccountsState,
    event: Ev,
    after: Vec<&'static str>,
    effects: Vec<Fx>,
}

fn case(
    name: &'static str,
    before: AccountsState,
    event: Ev,
    after: Vec<&'static str>,
    effects: Vec<Fx>,
) -> Case {
    Case {
        name,
        before,
        event,
        after,
        effects,
    }
}

/// Each row: what was known, the event, the accounts needing sign-in after it, the effects.
#[test]
fn events_move_the_list_and_call_for_effects() {
    let cases = vec![
        case(
            "a new account is told",
            known(&[]),
            Ev::NeedsSignIn(needing("a", "Work")),
            vec!["a"],
            vec![Fx::Tell(needing("a", "Work"))],
        ),
        case(
            "an account already known is not told twice",
            known(&["a"]),
            Ev::NeedsSignIn(needing("a", "a")),
            vec!["a"],
            vec![],
        ),
        case(
            "a second account is told beside the first",
            known(&["a"]),
            Ev::NeedsSignIn(needing("b", "b")),
            vec!["a", "b"],
            vec![Fx::Tell(needing("b", "b"))],
        ),
        case(
            "an account well again is withdrawn",
            known(&["a", "b"]),
            Ev::SignedIn(AccountKey::new("a")),
            vec!["b"],
            vec![Fx::Withdraw(AccountKey::new("a"))],
        ),
        case(
            "an account never known to need it changes nothing",
            known(&["a"]),
            Ev::SignedIn(AccountKey::new("z")),
            vec!["a"],
            vec![],
        ),
        case(
            "the list on meeting accountd tells the new and withdraws the gone",
            known(&["a", "b"]),
            Ev::Listed(vec![needing("b", "b"), needing("c", "c")]),
            vec!["b", "c"],
            vec![
                Fx::Withdraw(AccountKey::new("a")),
                Fx::Tell(needing("c", "c")),
            ],
        ),
        case(
            "a list of what is already known is silent",
            known(&["a"]),
            Ev::Listed(vec![needing("a", "a")]),
            vec!["a"],
            vec![],
        ),
    ];
    for Case {
        name,
        before,
        event,
        after,
        effects,
    } in cases
    {
        let (state, did) = step(before, event);
        assert_eq!(keys(&state), after, "{name}: the list");
        assert_eq!(did, effects, "{name}: the effects");
    }
}

#[test]
fn accountd_leaving_keeps_what_the_person_was_told_and_its_return_follows_the_bus() {
    let (gone, effects) = step(known(&["a"]), Ev::Gone);
    assert!(!gone.present);
    assert_eq!(keys(&gone), vec!["a"]);
    assert!(effects.is_empty());
    let (back, effects) = step(gone, Ev::Found);
    assert!(back.present);
    assert!(effects.is_empty());
}
