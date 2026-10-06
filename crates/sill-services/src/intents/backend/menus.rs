//! `shell.menu.list` and `shell.menu.activate` as the provider answers them.
//!
//! The listing is the app's own text, so it is labelled untrusted with the app as its source and
//! never joins the shell's own words. An activation answers in fixed words, never the item's.

use docket_client::DbusTransport;
use docket_core::{
    AppRefusal, FailText, Follow, Invocation, LabelText, Outcome, Preview, Undoable, Value,
};
use porter_core::{AppName, DataClass};
use prov::{Label, Labelled, Source};
use sill_model::intents::shell::MenuAsk;

use crate::app_menu::{Activated, MenuAgent, MenuFault};

/// The app a listing is attributed to when the window's `app_id` is no reverse-DNS name (`foot`):
/// the shell, as the one that read it. The label stays untrusted.
const SHELL_APP: &str = "org.quire.Shell";

pub(super) async fn perform(
    agent: &MenuAgent<DbusTransport>,
    ask: MenuAsk,
    inv: &Invocation,
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
                .activate(window.as_deref(), &item, inv.origin)
                .await
                .map_err(refusal)?;
            Ok(outcome(None, Some(said(&done.how))))
        }
    }
}

fn said(done: &Activated) -> &'static str {
    match done {
        Activated::Own => "Did the menu item",
        Activated::Action(_) => "Ran the menu item's action",
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
        MenuFault::Refused(why) => failed(&format!("the action was refused: {why}")),
        MenuFault::Failed(why) => failed(&why),
    }
}
