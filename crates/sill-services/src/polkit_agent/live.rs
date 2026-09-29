//! The agent's loop.
//!
//! Stays passive (publishes `Registration::Passive`, touches no bus) until `Policy(Own)`; then
//! connects to `setup.bus`, serves [`super::dbus::Agent`] at [`super::AGENT_PATH`], finds the
//! session id (`SessionChoice`: logind's `session/auto` `Id`), and registers
//! ([`super::bus`]); resolves the helper route once (`HelperChoice`,
//! [`super::helper::helper_route`]); turns every `Call`, command and helper line into a
//! [`super::PolkitEvent`] through [`super::reduce`], choosing the identity with
//! [`super::pick_identity`] and naming it from `/etc/passwd`; runs one helper per `StartHelper`
//! ([`super::run`]: a child with `kill_on_drop`, or the socket); keeps the answer from `Answer`
//! only until `SendAnswer` hands it to the run (dropped, and so zeroed, otherwise); answers the
//! D-Bus call on `Complete`; runs a `Fake` request's helper in-process (accepting
//! [`super::FAKE_SECRET`]); unregisters on `Policy(Off)` and when every control is gone;
//! publishes [`super::polkit_of`].

use std::collections::{HashMap, VecDeque};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use tokio::sync::{mpsc, oneshot, watch};

use sill_model::auth::{Secret, UserName};

use super::bus::{self, BusLink, BusTrouble};
use super::dbus::Call;
use super::helper::{HelperRoute, helper_route};
use super::identity::{name_in_passwd, pick_identity};
use super::machine::{Completion, Origin, PolkitEffect, PolkitEvent, PolkitState, polkit_of, step};
use super::run::{self, Driver, Run, RunEvent, RunId, RunSaid};
use super::types::{AuthRequest, Cookie, FakeRequest, Identity, Polkit, PolkitCommand, Uid};
use super::{HelperChoice, PolkitRoute};

pub(super) async fn run(
    tx: watch::Sender<Polkit>,
    mut commands: mpsc::UnboundedReceiver<PolkitCommand>,
    setup: PolkitRoute,
) {
    let (calls_tx, mut calls) = mpsc::unbounded_channel();
    let (events_tx, mut events) = mpsc::unbounded_channel();
    let mut live = Live::new(tx, setup, calls_tx, events_tx);
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command).await,
                None => break,
            },
            Some(call) = calls.recv() => live.call(call).await,
            Some(event) = events.recv() => live.run_event(event).await,
        }
    }
    live.feed(PolkitEvent::Policy(super::AgentPolicy::Off), None)
        .await;
    live.replies.clear();
}

/// This session's user.
#[derive(Debug, Clone)]
struct Own {
    uid: Uid,
    name: UserName,
}

struct Live {
    state: PolkitState,
    tx: watch::Sender<Polkit>,
    setup: PolkitRoute,
    own: Own,
    route: Option<HelperRoute>,
    bus: Option<BusLink>,
    replies: HashMap<Cookie, oneshot::Sender<Completion>>,
    current: Option<Run>,
    serial: u64,
    calls: mpsc::UnboundedSender<Call>,
    events: mpsc::UnboundedSender<RunEvent>,
}

impl Live {
    fn new(
        tx: watch::Sender<Polkit>,
        setup: PolkitRoute,
        calls: mpsc::UnboundedSender<Call>,
        events: mpsc::UnboundedSender<RunEvent>,
    ) -> Live {
        Live {
            state: PolkitState::default(),
            tx,
            setup,
            own: own(),
            route: None,
            bus: None,
            replies: HashMap::new(),
            current: None,
            serial: 0,
            calls,
            events,
        }
    }

    async fn command(&mut self, command: PolkitCommand) {
        match command {
            PolkitCommand::Policy(policy) => self.feed(PolkitEvent::Policy(policy), None).await,
            PolkitCommand::Answer { cookie, secret } => {
                self.feed(PolkitEvent::Answered(cookie), Some(secret)).await;
            }
            PolkitCommand::Dismiss(cookie) => self.feed(PolkitEvent::Dismissed(cookie), None).await,
            PolkitCommand::Fake(fake) => {
                let request = self.made_up(fake);
                let user = Some(self.own.name.clone());
                let origin = Origin::Fake;
                self.feed(
                    PolkitEvent::Begin {
                        request,
                        user,
                        origin,
                    },
                    None,
                )
                .await;
            }
        }
    }

    async fn call(&mut self, call: Call) {
        match call {
            Call::Begin { request, reply } => {
                let taken = self.replies.contains_key(&request.cookie)
                    || self
                        .state
                        .queue
                        .iter()
                        .any(|p| p.request.cookie == request.cookie);
                if taken {
                    let _ = reply.send(Completion::Failed);
                    return;
                }
                let user = user_for(&request.identities, &self.own);
                self.replies.insert(request.cookie.clone(), reply);
                let origin = Origin::Bus;
                self.feed(
                    PolkitEvent::Begin {
                        request,
                        user,
                        origin,
                    },
                    None,
                )
                .await;
            }
            Call::Cancel(cookie) => self.feed(PolkitEvent::Cancel(cookie), None).await,
        }
    }

    async fn run_event(&mut self, event: RunEvent) {
        let current = self
            .current
            .as_ref()
            .is_some_and(|run| run.id == event.run && run.cookie == event.cookie);
        if !current {
            return;
        }
        let event = match event.said {
            RunSaid::Line(line) => PolkitEvent::Helper(event.cookie, line),
            RunSaid::Ended => PolkitEvent::HelperEnded(event.cookie),
        };
        self.feed(event, None).await;
    }

    /// Reduce `first` and whatever its effects report, then publish. `held` is the answer of an
    /// `Answered`, handed on by its `SendAnswer` or dropped here.
    async fn feed(&mut self, first: PolkitEvent, mut held: Option<Secret>) {
        let mut pending = VecDeque::from([first]);
        while let Some(event) = pending.pop_front() {
            let (state, effects) = step(std::mem::take(&mut self.state), event);
            self.state = state;
            for effect in effects {
                pending.extend(self.perform(effect, &mut held).await);
            }
        }
        drop(held);
        self.tx.send_replace(polkit_of(&self.state));
    }

    async fn perform(
        &mut self,
        effect: PolkitEffect,
        held: &mut Option<Secret>,
    ) -> Option<PolkitEvent> {
        match effect {
            PolkitEffect::Register => return Some(self.register().await),
            PolkitEffect::Unregister => {
                if let Some(link) = &self.bus {
                    bus::unregister(link).await;
                }
            }
            PolkitEffect::StartHelper { cookie, user } => self.start(cookie, &user),
            PolkitEffect::SendAnswer(cookie) => {
                if let (Some(run), Some(secret)) = (&self.current, held.take())
                    && run.cookie == cookie
                {
                    run.answer(secret);
                }
            }
            PolkitEffect::StopHelper(cookie) => self.stop(&cookie),
            PolkitEffect::Complete {
                cookie, completion, ..
            } => {
                self.stop(&cookie);
                if let Some(reply) = self.replies.remove(&cookie) {
                    let _ = reply.send(completion);
                }
            }
        }
        None
    }

    async fn register(&mut self) -> PolkitEvent {
        if self.bus.is_none() {
            match bus::connect(&self.setup, self.calls.clone()).await {
                Ok(link) => self.bus = Some(link),
                Err(trouble) => return trouble_event(trouble),
            }
        }
        let Some(link) = &self.bus else {
            return PolkitEvent::BusGone;
        };
        match bus::register(link).await {
            Ok(()) => PolkitEvent::Registered,
            Err(trouble) => trouble_event(trouble),
        }
    }

    fn start(&mut self, cookie: Cookie, user: &UserName) {
        let origin = self
            .state
            .queue
            .iter()
            .find(|p| p.request.cookie == cookie)
            .map(|p| p.origin);
        let driver = match origin {
            Some(Origin::Fake) => Driver::StandIn,
            Some(Origin::Bus) | None => Driver::Helper(self.route().clone()),
        };
        self.serial += 1;
        let id = RunId(self.serial);
        self.current = Some(run::start(&driver, cookie, user, id, self.events.clone()));
    }

    fn stop(&mut self, cookie: &Cookie) {
        if self
            .current
            .as_ref()
            .is_some_and(|run| run.cookie == *cookie)
        {
            self.current = None;
        }
    }

    /// The helper route, resolved once.
    fn route(&mut self) -> &HelperRoute {
        let choice = &self.setup.helper;
        self.route.get_or_insert_with(|| match choice {
            HelperChoice::System => helper_route(Path::exists),
            HelperChoice::Use(route) => route.clone(),
        })
    }

    fn made_up(&mut self, fake: FakeRequest) -> AuthRequest {
        self.serial += 1;
        AuthRequest {
            cookie: Cookie(format!("sill-fake-{}", self.serial)),
            action_id: fake.action_id,
            message: fake.message,
            icon: fake.icon,
            details: Vec::new(),
            identities: vec![Identity::UnixUser(self.own.uid)],
        }
    }
}

fn trouble_event(trouble: BusTrouble) -> PolkitEvent {
    match trouble {
        BusTrouble::NoBus => PolkitEvent::BusGone,
        BusTrouble::Refused(message) => PolkitEvent::RegisterFailed(message),
    }
}

/// The identity to ask for, named; `None` when polkitd offers no user, or the chosen one has no
/// name in `/etc/passwd`.
fn user_for(identities: &[Identity], own: &Own) -> Option<UserName> {
    let uid = pick_identity(identities, own.uid)?;
    if uid == own.uid {
        return Some(own.name.clone());
    }
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    name_in_passwd(&passwd, uid)
}

/// This process's user: its uid (the owner of `/proc/self`), named from `/etc/passwd`, else
/// `$USER`.
fn own() -> Own {
    let uid = Uid(std::fs::metadata("/proc/self").map_or(u32::MAX, |m| m.uid()));
    let name = std::fs::read_to_string("/etc/passwd")
        .ok()
        .and_then(|passwd| name_in_passwd(&passwd, uid))
        .or_else(|| std::env::var("USER").ok().map(UserName))
        .unwrap_or_default();
    Own { uid, name }
}
