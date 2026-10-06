//! The window and workspace actions as the provider answers them: read the compositor's state, let
//! `sill_model::intents::shell::plan` decide, send its commands to the compositor service.
//!
//! The listings carry app-controlled titles, so they are labelled untrusted with the shell as the
//! reader. An action answers in fixed words, never a title's.

use docket_core::{
    AppRefusal, FailText, Follow, Invocation, LabelText, Outcome, Preview, Undoable, Value,
};
use porter_core::{AppName, DataClass};
use prov::{Label, Labelled, Source};
use sill_model::intents::shell::{WindowAnswer, WindowAsk, WindowFault, plan};

use crate::app_menu::MenuLinks;

/// The app a listing is attributed to: the shell, as the one that read the compositor.
const SHELL_APP: &str = "org.quire.Shell";

pub(super) fn perform(
    links: &MenuLinks,
    ask: &WindowAsk,
    inv: &Invocation,
) -> Result<Outcome, AppRefusal> {
    let wl = links.toplevels.borrow().clone();
    match plan(ask, &wl).map_err(refusal)? {
        WindowAnswer::Listing(text) => {
            let app = AppName::parse(SHELL_APP).map_err(|_| failed("the shell cannot be named"))?;
            let label = Label::untrusted(Source::App(app), DataClass::Screen, inv.space.clone());
            Ok(outcome(
                Some(Labelled::new(Value::Text(text), label)),
                ask.said(),
            ))
        }
        WindowAnswer::Commands(commands) => {
            for command in commands {
                links
                    .wl
                    .send(command)
                    .map_err(|_| failed("the compositor is not there"))?;
            }
            Ok(outcome(None, ask.said()))
        }
    }
}

fn outcome(value: Option<Labelled<Value>>, said: &str) -> Outcome {
    Outcome {
        value,
        said: LabelText::parse(said).ok(),
        show: Preview::None,
        undo: Undoable::No,
        follow: Follow::Nothing,
    }
}

fn failed(words: &str) -> AppRefusal {
    AppRefusal::Failed(FailText(words.to_owned()))
}

fn refusal(fault: WindowFault) -> AppRefusal {
    match fault {
        WindowFault::NoSuchWindow => failed("there is no window with that id"),
        WindowFault::NoSuchWorkspace => failed("there is no workspace with that id or position"),
        WindowFault::NoSuchApp => failed("there is no window of that app"),
    }
}
