use crate::notifications::ActionKey;

use super::{
    Ask, Computer, GuestAsksEffect as Fx, GuestAsksEvent as Ev, GuestAsksState, Verdict,
    choice_key, choice_of, notice_text, step,
};

fn ask(node: &str, name: &str) -> Ask {
    Ask {
        node: Computer::new(node),
        name: name.to_owned(),
    }
}

fn run(events: Vec<Ev>) -> (GuestAsksState, Vec<Fx>) {
    events.into_iter().fold(
        (GuestAsksState::default(), Vec::new()),
        |(state, mut said), event| {
            let (next, effects) = step(state, event);
            said.extend(effects);
            (next, said)
        },
    )
}

#[test]
fn a_question_is_told_once_answered_once_and_withdrawn_when_answered_elsewhere() {
    let (state, said) = run(vec![
        Ev::Asked(ask("n1", "Laptop")),
        Ev::Asked(ask("n1", "Laptop")),
        Ev::Waiting(vec![ask("n1", "Laptop"), ask("n2", "Studio")]),
        Ev::Chosen(Computer::new("n1"), Verdict::Allow),
        Ev::Chosen(Computer::new("n1"), Verdict::Deny),
        Ev::Waiting(vec![]),
    ]);
    assert_eq!(state, GuestAsksState::default());
    assert_eq!(
        said,
        vec![
            Fx::Tell(ask("n1", "Laptop")),
            Fx::Tell(ask("n2", "Studio")),
            Fx::Withdraw(Computer::new("n1")),
            Fx::Answer(Computer::new("n1"), Verdict::Allow),
            Fx::Withdraw(Computer::new("n2")),
        ]
    );
}

#[test]
fn the_words_name_the_computer_and_each_button_carries_its_verdict() {
    let notice = notice_text(&ask("n1", "Laptop"));
    assert_eq!(notice.summary, "Let Laptop use this computer's models?");
    assert_eq!(
        (notice.allow.as_str(), notice.deny.as_str()),
        ("Allow", "Deny")
    );
    let node = Computer::new("n1");
    for verdict in [Verdict::Allow, Verdict::Deny] {
        assert_eq!(
            choice_of(&choice_key(&node, verdict)),
            Some((node.clone(), verdict))
        );
    }
    assert_eq!(choice_of(&ActionKey("open-settings:x".to_owned())), None);
}
