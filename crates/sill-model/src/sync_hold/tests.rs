use crate::notifications::ActionKey;
use sill_launcher::settings_target::{AccountRef, SettingsTarget};

use super::{
    Dataset, Hold, SyncHoldEffect as Fx, SyncHoldEvent as Ev, SyncHoldState, dataset_of,
    notice_text, open_key, step,
};

fn hold(dataset: &str, discard: u64, held: u64) -> Hold {
    Hold {
        dataset: Dataset::new(dataset),
        account_name: "Work files".to_owned(),
        discard,
        held,
        account: SettingsTarget::Account(AccountRef(
            "/org/quire/Accounts1/account/work".to_owned(),
        )),
    }
}

#[test]
fn a_hold_is_told_replaced_on_a_repeat_and_withdrawn_when_it_ends() {
    let run = |events: Vec<Ev>| {
        events.into_iter().fold(
            (SyncHoldState::default(), Vec::new()),
            |(state, mut said), event| {
                let (next, effects) = step(state, event);
                said.extend(effects);
                (next, said)
            },
        )
    };
    let (state, said) = run(vec![
        Ev::Held(hold("work/files", 30, 40)),
        Ev::Held(hold("work/files", 31, 40)),
        Ev::Ended(Dataset::new("other/files")),
        Ev::Ended(Dataset::new("work/files")),
    ]);
    assert_eq!(state, SyncHoldState::default());
    assert_eq!(
        said,
        vec![
            Fx::Tell(hold("work/files", 30, 40)),
            Fx::Tell(hold("work/files", 31, 40)),
            Fx::Withdraw(Dataset::new("work/files")),
        ]
    );
}

#[test]
fn the_words_name_the_account_and_the_counts_and_offer_no_removal() {
    let some = notice_text(&hold("work/files", 30, 40));
    assert_eq!(some.summary, "Files removed online in Work files");
    assert_eq!(
        some.body,
        "30 of 40 files were removed online. Open Settings to decide."
    );
    assert_eq!(some.button, "Open Settings");
    let all = notice_text(&hold("work/files", 40, 40));
    assert_eq!(
        all.body,
        "All 40 files were removed online. Open Settings to decide."
    );
}

#[test]
fn an_action_key_carries_its_dataset_and_no_other_key_is_one() {
    let dataset = Dataset::new("work/files");
    let key = open_key(&dataset);
    assert_eq!(dataset_of(&key), Some(dataset.clone()));
    assert_eq!(dataset_of(&ActionKey("sign-in-again:x".to_owned())), None);
}
