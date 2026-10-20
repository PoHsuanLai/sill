//! Actions, from every surface to the daemon: a surface names an action and what it is done to,
//! says where it asked from, and the daemon's interpreter (`sill::actions`) carries it out. The
//! surface never picks an output, fetches a token or decides to ask first: the daemon does, once.

use shell_host::{OutputId, SurfaceHandle, SurfaceId};
use sill_launcher::actions::ActionId;
use sill_model::actions::{Ask, Subject};
use tokio::sync::watch;

use super::{Outbox, Sent};

/// Where an ask came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Origin {
    /// The surface the person used: an app it launches gets that surface's activation token.
    pub surface: Option<SurfaceId>,
    /// The output it is on: a panel the action opens opens there. None: the daemon picks.
    pub output: Option<OutputId>,
}

impl Origin {
    /// An ask from `surface` (when it is still there) on `output`.
    pub fn of(surface: Option<&SurfaceHandle>, output: OutputId) -> Origin {
        Origin {
            surface: surface.map(SurfaceHandle::id),
            output: Some(output),
        }
    }

    /// An ask from nowhere in particular: the command line, an agent.
    pub fn anywhere() -> Origin {
        Origin::default()
    }
}

/// One ask and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub ask: Ask,
    pub origin: Origin,
}

/// An ask, numbered.
pub type ActionSent = Sent<Asked>;

/// The surfaces' half and the daemon's receiver. Clone is cheap.
#[derive(Debug, Clone, Default)]
pub struct ActionLink {
    asked: Outbox<Asked>,
}

impl ActionLink {
    /// Nothing asked yet.
    pub fn new() -> ActionLink {
        ActionLink::default()
    }

    /// Ask for `ask` from `origin`.
    pub fn ask(&self, ask: Ask, origin: Origin) {
        self.asked.send(Asked { ask, origin });
    }

    /// Ask for `action` on the desktop itself.
    pub fn do_it(&self, action: ActionId, origin: Origin) {
        self.ask(Ask::of(action), origin);
    }

    /// Ask for `action` done to `subject`.
    pub fn do_to(&self, action: ActionId, subject: Subject, origin: Origin) {
        self.ask(Ask::on(action, subject), origin);
    }

    /// The latest ask.
    pub fn last(&self) -> Option<ActionSent> {
        self.asked.latest()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe(&self) -> watch::Receiver<Option<ActionSent>> {
        self.asked.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use sill_launcher::actions::ActionId;

    use super::{ActionLink, Origin};

    #[test]
    fn every_ask_is_news_even_the_same_action_twice() {
        let link = ActionLink::new();
        let follower = link.subscribe();
        assert_eq!(link.last(), None);
        link.do_it(ActionId::Lock, Origin::anywhere());
        link.do_it(ActionId::Lock, Origin::anywhere());
        let last = link.last().expect("asked");
        assert_eq!((last.seq, last.ask.action), (2, ActionId::Lock));
        assert_eq!(follower.has_changed().ok(), Some(true));
    }
}
