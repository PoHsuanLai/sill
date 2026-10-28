//! `shell.menu.list` and `shell.menu.activate` as the provider answers them.
//!
//! The listing is the app's own text, so it is labelled untrusted with the app as its source and
//! never joins the shell's own words. An activation answers in fixed words, never the item's.

use docket_client::Transport;
use docket_core::{AppRefusal, FailText, Follow, Invocation, Outcome, Preview, Undoable};
use docket_vocab::{CallClass, LabelText, Value};
use porter_core::{AppName, DataClass};
use prov::{Label, Labelled, Source};
use sill_model::intents::shell::MenuAsk;

use crate::app_menu::{Activated, MenuAgent, MenuFault};

/// The app a listing is attributed to when the window's `app_id` is no reverse-DNS name (`foot`):
/// the shell, as the one that read it. The label stays untrusted.
const SHELL_APP: &str = "org.quire.Shell";

/// What `ask` is, for the router's `Classify`. Only an activation has a class.
pub async fn classify<T: Transport>(
    agent: &MenuAgent<T>,
    ask: MenuAsk,
) -> Result<CallClass, AppRefusal> {
    match ask {
        MenuAsk::Activate { window, item } => agent
            .classify(window.as_deref(), &item)
            .await
            .map_err(refusal),
        MenuAsk::List { .. } => Err(AppRefusal::Unsupported),
    }
}

/// What `ask` does, given the class the router gated it on.
pub async fn perform<T: Transport>(
    agent: &MenuAgent<T>,
    ask: MenuAsk,
    inv: &Invocation,
    classified: Option<CallClass>,
) -> Result<Outcome, AppRefusal> {
    match ask {
        MenuAsk::List { window } => {
            let listed = agent.list(window.as_deref()).await.map_err(refusal)?;
            let source = AppName::parse(&listed.app.0)
                .or_else(|_| AppName::parse(SHELL_APP))
                .map_err(|_| failed("the app cannot be named"))?;
            let label = Label::untrusted(Source::App(source), DataClass::Screen, inv.space.clone());
            Ok(outcome(
                Some(Labelled::new(Value::Text(listed.text), label)),
                None,
            ))
        }
        MenuAsk::Activate { window, item } => {
            let done = agent
                .activate(window.as_deref(), &item, classified.as_ref())
                .await
                .map_err(refusal)?;
            let mut out = outcome(None, Some(said(&done.how)));
            if let Activated::Delegated(call) = done.how {
                out.follow = Follow::Next(call);
            }
            Ok(out)
        }
    }
}

fn said(done: &Activated) -> &'static str {
    match done {
        Activated::Own => "Did the menu item",
        Activated::Delegated(_) => "Passed the menu item to the app's own action",
        Activated::Clicked => "Clicked the menu item",
    }
}

fn outcome(value: Option<Labelled<Value>>, said: Option<&str>) -> Outcome {
    Outcome {
        value,
        said: said.and_then(|words| LabelText::parse(words).ok()),
        show: Preview::None,
        undo: Undoable::No,
        follow: Follow::Nothing,
    }
}

fn failed(words: &str) -> AppRefusal {
    AppRefusal::Failed(FailText(words.to_owned()))
}

fn refusal(fault: MenuFault) -> AppRefusal {
    match fault {
        MenuFault::NoWindow => failed("there is no such window"),
        MenuFault::NoMenu => failed("the window's app has no menu"),
        MenuFault::NoSuchItem => failed("the menu has no such item"),
        MenuFault::NotRunnable => failed("the menu item is disabled or opens a submenu"),
        MenuFault::Changed => AppRefusal::ClassificationChanged,
        MenuFault::Failed(why) => failed(&why),
    }
}
