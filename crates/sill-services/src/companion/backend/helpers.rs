//! The helpers on `org.quire.Companion1.Helpers2`: the calls the helpers screen asks for (the
//! helpers, signing in to the apps they run, their routines), and the `Changed` signal that tells
//! it to read the list again. Every body is JSON of `companion-wire`'s helper types; a refusal is
//! the companion's own sentence and the model shows it as it is.

use companion_wire::{HelperCardView, HelperForm, HelperReply, HelpersView};
use docket_dbus::{COMPANION_BUS, CompanionHelpers2Proxy};
use porter_client::DbusTransport;
use prov::HelperId;
use sill_model::companion::{Asked, CompanionEvent, HelpersCommand, HelpersMsg, model_groups};
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;

use super::text::{from_json, text, to_json};
use super::{pool, routines};
use crate::owner;
use crate::stream::next;

/// What the companion's helpers answered to `command`, as the message the screen takes; none for
/// a call that says nothing back (the `Changed` signal follows it). A call that could not be made
/// or understood is `Unreached`, and the cause is logged.
pub(super) async fn carry(connection: &Connection, command: HelpersCommand) -> Option<HelpersMsg> {
    let proxy = match CompanionHelpers2Proxy::new(connection).await {
        Ok(proxy) => proxy,
        Err(error) => return unreached(&error),
    };
    match command {
        HelpersCommand::List { after } => Some(read_list(&proxy, after).await),
        HelpersCommand::ReadCard(helper) => Some(read_card(&proxy, helper).await),
        HelpersCommand::Create(form) => Some(save(&proxy, None, &form).await),
        HelpersCommand::Edit { helper, form } => Some(save(&proxy, Some(&helper), &form).await),
        HelpersCommand::Pause(helper) => silent(proxy.pause(helper.as_str()).await),
        HelpersCommand::Resume(helper) => silent(proxy.resume(helper.as_str()).await),
        HelpersCommand::Remove(helper) => silent(proxy.remove(helper.as_str()).await),
        HelpersCommand::PostJob { helper, goal } => {
            // The things only this job may change: the person's own jobs name none.
            let posted = proxy.post_job(helper.as_str(), &goal, "[]").await;
            Some(reply(posted, Asked::Job))
        }
        HelpersCommand::SignIn { helper, way } => {
            let answer = proxy.sign_in(helper.as_str(), &way).await;
            Some(reply(answer, Asked::SignIn))
        }
        HelpersCommand::SignOut(helper) => {
            let answer = proxy.sign_out(helper.as_str()).await;
            Some(reply(answer, Asked::SignOut))
        }
        HelpersCommand::OpenInTerminal(helper) => {
            let answer = proxy.open_in_terminal(helper.as_str()).await;
            Some(reply(answer, Asked::OpenInTerminal))
        }
        HelpersCommand::BringBack(helper) => {
            let answer = proxy.bring_back(helper.as_str()).await;
            Some(reply(answer, Asked::BringBack))
        }
        HelpersCommand::BringBackAfterClose(helper) => {
            let answer = proxy.bring_back(helper.as_str()).await;
            Some(reply(answer, Asked::BringBackAfterClose))
        }
        HelpersCommand::Routines(command) => routines::carry(&proxy, command).await,
        HelpersCommand::ReadModels => Some(read_models(connection).await),
        HelpersCommand::SetPool { helper, pool } => {
            Some(pool::keep(connection, &helper, &pool).await)
        }
        // The launch service opens it, and the loop opens the sign-in window; neither is handed
        // here.
        HelpersCommand::OpenModelPicker | HelpersCommand::OpenTerminal(_) => None,
    }
}

/// The models the places serve, grouped; this computer's under the engine that runs each.
async fn read_models(connection: &Connection) -> HelpersMsg {
    let transport = DbusTransport::over(connection.clone());
    match transport.list_places().await {
        Ok(read) => HelpersMsg::ModelsOffered(model_groups(&read.rows)),
        Err(why) => unreached(&why).unwrap_or(HelpersMsg::Unreached),
    }
}

async fn read_list(proxy: &CompanionHelpers2Proxy<'_>, after: u64) -> HelpersMsg {
    match proxy
        .list(after)
        .await
        .map_err(text)
        .and_then(|body| from_json::<HelpersView>(&body))
    {
        Ok(view) => HelpersMsg::Listed(view),
        Err(why) => unreached(&why).unwrap_or(HelpersMsg::Unreached),
    }
}

/// A card: a helper the companion does not have is `CardMissing`; a companion that cannot be
/// reached is not.
async fn read_card(proxy: &CompanionHelpers2Proxy<'_>, helper: HelperId) -> HelpersMsg {
    match proxy.card(helper.as_str()).await {
        Ok(body) => match from_json::<HelperCardView>(&body) {
            Ok(view) => HelpersMsg::CardRead(Box::new(view)),
            Err(why) => unreached(&why).unwrap_or(HelpersMsg::Unreached),
        },
        Err(zbus::Error::MethodError(..)) => HelpersMsg::CardMissing(helper),
        Err(error) => unreached(&error).unwrap_or(HelpersMsg::Unreached),
    }
}

/// Make a helper (`helper` is `None`) or change one.
async fn save(
    proxy: &CompanionHelpers2Proxy<'_>,
    helper: Option<&HelperId>,
    form: &HelperForm,
) -> HelpersMsg {
    let body = match to_json(form) {
        Ok(body) => body,
        Err(why) => return unreached(&why).unwrap_or(HelpersMsg::Unreached),
    };
    let saved = match helper {
        Some(helper) => proxy.edit(helper.as_str(), &body).await,
        None => proxy.create(&body).await,
    };
    reply(saved, Asked::Save)
}

/// A call's `HelperReply` as the message that carries it to the model.
fn reply(answer: zbus::Result<String>, asked: Asked) -> HelpersMsg {
    match answer
        .map_err(text)
        .and_then(|body| from_json::<HelperReply>(&body))
    {
        Ok(reply) => HelpersMsg::Replied(asked, reply),
        Err(why) => unreached(&why).unwrap_or(HelpersMsg::Unreached),
    }
}

/// A call with no body: nothing to tell the model unless it failed.
pub(super) fn silent(done: zbus::Result<()>) -> Option<HelpersMsg> {
    match done {
        Ok(()) => None,
        Err(error) => unreached(&error),
    }
}

/// Log what went wrong; the person is told that the helpers could not be reached.
pub(super) fn unreached(why: &dyn std::fmt::Display) -> Option<HelpersMsg> {
    eprintln!("sill: companion: helpers: {why}");
    Some(HelpersMsg::Unreached)
}

/// The companion's helpers while it is on the bus: the list is read when it appears and each
/// time it says something changed. The signal is subscribed before the first read, so no change
/// falls between the two.
pub(super) async fn follow(connection: Connection, events: UnboundedSender<CompanionEvent>) {
    loop {
        owner::appeared(&connection, COMPANION_BUS).await;
        tokio::select! {
            () = changes(&connection, &events) => return,
            () = owner::changed(&connection, COMPANION_BUS) => {}
        }
    }
}

/// `Changed` as the model's message; ends when the events have no receiver.
async fn changes(connection: &Connection, events: &UnboundedSender<CompanionEvent>) {
    let Ok(proxy) = CompanionHelpers2Proxy::new(connection).await else {
        return std::future::pending().await;
    };
    let Ok(mut changed) = proxy.receive_changed().await else {
        return std::future::pending().await;
    };
    let tell = |msg| events.send(CompanionEvent::Helpers(msg));
    if tell(HelpersMsg::Changed).is_err() {
        return;
    }
    while next(&mut changed).await.is_some() {
        if tell(HelpersMsg::Changed).is_err() {
            return;
        }
    }
}
