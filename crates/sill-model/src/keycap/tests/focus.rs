use keycap_protocol::{MAX_PACKET, ProfileName, Steer, ToDaemon};

use crate::keycap::{Wants, focus_to_send, user_config};

fn name(s: &str) -> ProfileName {
    ProfileName::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
}

fn focus(s: &str) -> ToDaemon {
    ToDaemon::Steer(Steer::Focus { profile: name(s) })
}

fn config(s: &str) -> ToDaemon {
    ToDaemon::Steer(Steer::UserConfig { toml: s.to_owned() })
}

#[test]
fn focus_is_sent_only_when_the_profile_changes() {
    let cases: &[(&str, Option<&str>, &str, Option<&str>)] = &[
        ("nothing sent yet", None, "default", Some("default")),
        (
            "a new profile",
            Some("default"),
            "terminal",
            Some("terminal"),
        ),
        ("the same profile", Some("terminal"), "terminal", None),
        ("back again", Some("terminal"), "default", Some("default")),
    ];
    for &(case, previous, next, want) in cases {
        let previous = previous.map(name);
        assert_eq!(
            focus_to_send(previous.as_ref(), name(next)),
            want.map(name),
            "{case}"
        );
    }
}

#[test]
fn a_config_that_fits_in_one_packet_is_sent() {
    let cases: Vec<(&str, String, Option<ToDaemon>)> = vec![
        ("empty", String::new(), Some(config(""))),
        (
            "small",
            "[profile.term]\n".to_owned(),
            Some(config("[profile.term]\n")),
        ),
        ("one packet and more", "#".repeat(MAX_PACKET), None),
    ];
    for (case, toml, want) in cases {
        assert_eq!(user_config(toml), want, "{case}");
    }
}

#[test]
fn wants_remember_the_last_of_each_and_greet_with_both() {
    let steps = [
        (
            focus("default"),
            Wants {
                focus: Some(name("default")),
                config: None,
            },
        ),
        (
            config("a"),
            Wants {
                focus: Some(name("default")),
                config: Some("a".to_owned()),
            },
        ),
        (
            focus("terminal"),
            Wants {
                focus: Some(name("terminal")),
                config: Some("a".to_owned()),
            },
        ),
        (
            config("b"),
            Wants {
                focus: Some(name("terminal")),
                config: Some("b".to_owned()),
            },
        ),
        (
            ToDaemon::Steer(Steer::Resume),
            Wants {
                focus: Some(name("terminal")),
                config: Some("b".to_owned()),
            },
        ),
    ];
    let mut wants = Wants::default();
    for (message, want) in steps {
        wants = wants.after(&message);
        assert_eq!(wants, want, "{message:?}");
    }
    assert_eq!(wants.greeting(), vec![focus("terminal"), config("b")]);
}

#[test]
fn a_greeting_holds_only_what_was_wanted() {
    let cases: Vec<(&str, Wants, Vec<ToDaemon>)> = vec![
        ("nothing", Wants::default(), vec![]),
        (
            "a focus",
            Wants {
                focus: Some(name("default")),
                config: None,
            },
            vec![focus("default")],
        ),
        (
            "a config",
            Wants {
                focus: None,
                config: Some("x".to_owned()),
            },
            vec![config("x")],
        ),
    ];
    for (case, wants, want) in cases {
        assert_eq!(wants.greeting(), want, "{case}");
    }
}
