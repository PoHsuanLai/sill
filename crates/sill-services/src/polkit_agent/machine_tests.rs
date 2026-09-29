use super::{
    Completion, MAX_TRIES, Origin, PolkitEffect, PolkitEvent, PolkitState, polkit_of, step,
};
use crate::polkit_agent::helper::HelperLine;
use crate::polkit_agent::types::{
    ActionId, AgentPolicy, AuthRequest, Cookie, Echo, Identity, LastTry, PromptPhase, Registration,
    Uid,
};
use sill_model::auth::{PamNote, UserName};

fn cookie(n: u32) -> Cookie {
    Cookie(format!("cookie-{n}"))
}

fn ada() -> UserName {
    UserName("ada".to_owned())
}

fn request(n: u32) -> AuthRequest {
    AuthRequest {
        cookie: cookie(n),
        action_id: ActionId("org.example.pkexec.run".to_owned()),
        message: "Authentication is required to run a program".to_owned(),
        icon: String::new(),
        details: Vec::new(),
        identities: vec![Identity::UnixUser(Uid(1000))],
    }
}

fn begin(n: u32) -> PolkitEvent {
    PolkitEvent::Begin {
        request: request(n),
        user: Some(ada()),
        origin: Origin::Bus,
    }
}

fn start(n: u32) -> PolkitEffect {
    PolkitEffect::StartHelper {
        cookie: cookie(n),
        user: ada(),
    }
}

fn complete(n: u32, completion: Completion) -> PolkitEffect {
    PolkitEffect::Complete {
        cookie: cookie(n),
        origin: Origin::Bus,
        completion,
    }
}

fn registered() -> PolkitState {
    PolkitState {
        policy: AgentPolicy::Own,
        registration: Registration::Registered,
        queue: Vec::new(),
    }
}

/// One case: a name, the start, the events, the registration after, and the effects.
type RegistrationCase = (
    &'static str,
    PolkitState,
    Vec<PolkitEvent>,
    Registration,
    Vec<PolkitEffect>,
);

fn run(start: PolkitState, events: Vec<PolkitEvent>) -> (PolkitState, Vec<PolkitEffect>) {
    events
        .into_iter()
        .fold((start, Vec::new()), |(state, mut all), event| {
            let (state, effects) = step(state, event);
            all.extend(effects);
            (state, all)
        })
}

fn hidden_prompt(n: u32) -> PolkitEvent {
    PolkitEvent::Helper(cookie(n), HelperLine::PromptHidden("Password: ".to_owned()))
}

#[test]
fn registration_follows_the_policy() {
    let cases: Vec<RegistrationCase> = vec![
        (
            "own registers",
            PolkitState::default(),
            vec![
                PolkitEvent::Policy(AgentPolicy::Own),
                PolkitEvent::Registered,
            ],
            Registration::Registered,
            vec![PolkitEffect::Register],
        ),
        (
            "a refusal is kept",
            PolkitState::default(),
            vec![
                PolkitEvent::Policy(AgentPolicy::Own),
                PolkitEvent::RegisterFailed("already an agent".to_owned()),
            ],
            Registration::Refused("already an agent".to_owned()),
            vec![PolkitEffect::Register],
        ),
        (
            "off unregisters and cancels what waits",
            registered(),
            vec![begin(1), PolkitEvent::Policy(AgentPolicy::Off)],
            Registration::Passive,
            vec![
                start(1),
                PolkitEffect::Unregister,
                PolkitEffect::StopHelper(cookie(1)),
                complete(1, Completion::Cancelled),
            ],
        ),
    ];
    for (name, state, events, want_registration, want_effects) in cases {
        let (state, effects) = run(state, events);
        assert_eq!(state.registration, want_registration, "{name}");
        assert_eq!(effects, want_effects, "{name}");
    }
}

#[test]
fn one_request_through_its_helper() {
    let cases: Vec<(&str, Vec<PolkitEvent>, Vec<PolkitEffect>)> = vec![
        (
            "right the first time",
            vec![
                begin(1),
                hidden_prompt(1),
                PolkitEvent::Answered(cookie(1)),
                PolkitEvent::Helper(cookie(1), HelperLine::Success),
            ],
            vec![
                start(1),
                PolkitEffect::SendAnswer(cookie(1)),
                complete(1, Completion::Authorized),
            ],
        ),
        (
            "wrong twice then right",
            vec![
                begin(1),
                hidden_prompt(1),
                PolkitEvent::Answered(cookie(1)),
                PolkitEvent::Helper(cookie(1), HelperLine::Failure),
                hidden_prompt(1),
                PolkitEvent::Answered(cookie(1)),
                PolkitEvent::HelperEnded(cookie(1)),
                hidden_prompt(1),
                PolkitEvent::Answered(cookie(1)),
                PolkitEvent::Helper(cookie(1), HelperLine::Success),
            ],
            vec![
                start(1),
                PolkitEffect::SendAnswer(cookie(1)),
                start(1),
                PolkitEffect::SendAnswer(cookie(1)),
                start(1),
                PolkitEffect::SendAnswer(cookie(1)),
                complete(1, Completion::Authorized),
            ],
        ),
        (
            "an answer before the prompt is dropped",
            vec![begin(1), PolkitEvent::Answered(cookie(1))],
            vec![start(1)],
        ),
        (
            "the person cancels",
            vec![
                begin(1),
                hidden_prompt(1),
                PolkitEvent::Dismissed(cookie(1)),
            ],
            vec![
                start(1),
                PolkitEffect::StopHelper(cookie(1)),
                complete(1, Completion::Cancelled),
            ],
        ),
        (
            "nobody to ask for fails at once",
            vec![PolkitEvent::Begin {
                request: request(1),
                user: None,
                origin: Origin::Bus,
            }],
            vec![complete(1, Completion::Failed)],
        ),
        (
            "a stranger's cookie changes nothing",
            vec![begin(1), PolkitEvent::Cancel(cookie(9))],
            vec![start(1)],
        ),
    ];
    for (name, events, want) in cases {
        assert_eq!(run(registered(), events).1, want, "{name}");
    }
    let failures: Vec<PolkitEvent> = std::iter::once(begin(1))
        .chain((0..MAX_TRIES).flat_map(|_| {
            [
                hidden_prompt(1),
                PolkitEvent::Answered(cookie(1)),
                PolkitEvent::Helper(cookie(1), HelperLine::Failure),
            ]
        }))
        .collect();
    let (state, effects) = run(registered(), failures);
    assert!(state.queue.is_empty());
    assert_eq!(effects.last(), Some(&complete(1, Completion::Failed)));
}

#[test]
fn the_queue_and_the_view() {
    let (state, effects) = run(registered(), vec![begin(1), begin(2)]);
    assert_eq!(effects, vec![start(1)], "the second waits");
    let view = polkit_of(&state);
    assert_eq!(view.waiting, 1);
    assert_eq!(
        view.current.as_ref().map(|v| v.cookie.clone()),
        Some(cookie(1))
    );
    let (state, effects) = run(
        state,
        vec![
            PolkitEvent::Helper(cookie(1), HelperLine::Error("bad".to_owned())),
            PolkitEvent::Cancel(cookie(1)),
        ],
    );
    assert_eq!(
        effects,
        vec![
            PolkitEffect::StopHelper(cookie(1)),
            complete(1, Completion::Cancelled),
            start(2),
        ]
    );
    let view = polkit_of(&state);
    assert_eq!(view.waiting, 0);
    let current = view
        .current
        .unwrap_or_else(|| panic!("the second is shown"));
    assert_eq!(current.phase, PromptPhase::Starting);
    assert_eq!(current.last, LastTry::None);
    let (state, _) = run(
        state,
        vec![
            PolkitEvent::Helper(cookie(2), HelperLine::Info("touch".to_owned())),
            hidden_prompt(2),
        ],
    );
    let current = polkit_of(&state).current.unwrap_or_else(|| panic!("shown"));
    assert_eq!(current.notes, vec![PamNote::Info("touch".to_owned())]);
    assert_eq!(
        current.phase,
        PromptPhase::Asking {
            prompt: "Password: ".to_owned(),
            echo: Echo::Hidden,
        }
    );
}
