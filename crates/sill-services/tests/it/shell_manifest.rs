//! The shell's manifest (`dist/intents/org.quire.Shell.toml`) against the model that reads an
//! invocation of it: every declared action is one the model knows, every choice a declared
//! option, so a name or an option cannot drift between the file docket reads and the code sill
//! runs.

use std::collections::BTreeMap;

use docket_core::{
    AppRefusal, CallId, ChoiceId, Invocation, Origin, ParamDecl, ParamNeed, ParamType, Value,
};
use ds_core::word::Word;
use prov::{ActionName, Actor, Label, Labelled, SpaceId};
use sill_launcher::actions::ActionId;
use sill_model::actions::intent_of;
use sill_model::intents::shell::{action_of, menu_ask_of, window_ask_of};
use sill_services::intents::manifest;

/// An invocation of `action` with the arguments `args`.
fn invocation(action: &ActionName, args: BTreeMap<docket_core::ParamName, Value>) -> Invocation {
    Invocation {
        call: CallId(1),
        action: action.clone(),
        target: docket_core::TargetValue::Nothing,
        args: args
            .into_iter()
            .map(|(name, value)| {
                (
                    name,
                    Labelled {
                        value,
                        label: Label::trusted_user(),
                    },
                )
            })
            .collect(),
        actor: Actor::Cli,
        origin: Origin::Launcher,
        space: SpaceId::parse("desktop").unwrap_or_else(|e| panic!("{e}")),
    }
}

/// A value that fits `param`: its first option, or text for a text parameter.
fn value_for(param: &ParamDecl) -> Value {
    match &param.ty {
        ParamType::Choice(options) => Value::Choice(options[0].id.clone()),
        ParamType::Text { .. } => Value::Text("firefox.desktop".to_owned()),
        ParamType::Integer { min, .. } => Value::Integer(*min),
        other => panic!("{}: the shell declares no {other:?} parameter", param.name),
    }
}

/// An invocation read as the shell reads it: a menu ask, a window ask, or one of its other actions.
fn reads(invocation: &Invocation) -> Result<(), AppRefusal> {
    if menu_ask_of(invocation)?.is_some() || window_ask_of(invocation)?.is_some() {
        return Ok(());
    }
    action_of(invocation).map(drop)
}

#[test]
fn the_shipped_manifest_is_valid_and_is_the_shells() {
    let manifest = manifest().unwrap_or_else(|e| panic!("{e}"));
    let manifest = manifest.manifest();
    assert_eq!(manifest.app.as_str(), "org.quire.Shell");
    assert_eq!(manifest.actions.len(), 17);
}

#[test]
fn every_declared_action_reads_and_every_option_is_one_the_model_knows() {
    let manifest = manifest().unwrap_or_else(|e| panic!("{e}"));
    for action in &manifest.manifest().actions {
        // With every parameter given as its first option.
        let given = action
            .params
            .iter()
            .map(|p| (p.name.clone(), value_for(p)))
            .collect();
        let read = reads(&invocation(&action.name, given));
        assert!(read.is_ok(), "{}: {read:?}", action.name);
        // With none given: a defaulted or optional parameter is fine, a required one is asked for.
        let bare = reads(&invocation(&action.name, BTreeMap::new()));
        let needs = action
            .params
            .iter()
            .any(|p| matches!(p.need, ParamNeed::Required));
        match (needs, &bare) {
            (true, Err(AppRefusal::NeedsParam { .. })) | (false, Ok(_)) => {}
            _ => panic!("{}: {bare:?} with required params {needs}", action.name),
        }
        // Every declared option of every choice is read.
        for param in &action.params {
            if let ParamType::Choice(options) = &param.ty {
                for option in options {
                    let given = BTreeMap::from([(
                        param.name.clone(),
                        Value::Choice(
                            ChoiceId::parse(option.id.as_str()).unwrap_or_else(|e| panic!("{e}")),
                        ),
                    )]);
                    let read = reads(&invocation(&action.name, given));
                    assert!(read.is_ok(), "{} {}: {read:?}", action.name, option.id);
                }
            }
        }
    }
}

#[test]
fn the_manifest_and_the_shells_actions_name_each_other() {
    let manifest = manifest().unwrap_or_else(|e| panic!("{e}"));
    let declared: Vec<&str> = manifest
        .manifest()
        .actions
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    // Every action that names an intent names one the manifest declares.
    for id in ActionId::ALL {
        if let Some(name) = intent_of(*id) {
            assert!(declared.contains(&name), "{id:?} names {name}");
        }
    }
    // Every declared action the shell does as an action of its own (not a window, an app or a
    // menu ask, which take ids from a listing) is the intent of at least one of them. Only the
    // required parameters are given: an optional one can make it a listing ask
    // (`shell.workspace.switch` given a workspace).
    for action in &manifest.manifest().actions {
        let given = action
            .params
            .iter()
            .filter(|p| matches!(p.need, ParamNeed::Required))
            .map(|p| (p.name.clone(), value_for(p)))
            .collect();
        let call = invocation(&action.name, given);
        let own = matches!(menu_ask_of(&call), Ok(None))
            && matches!(window_ask_of(&call), Ok(None))
            && action_of(&call).is_ok();
        let named = ActionId::ALL
            .iter()
            .any(|id| intent_of(*id) == Some(action.name.as_str()));
        assert_eq!(own, named, "{}", action.name);
    }
}

#[test]
fn a_defaulted_choice_names_one_of_its_options() {
    let manifest = manifest().unwrap_or_else(|e| panic!("{e}"));
    for action in &manifest.manifest().actions {
        for param in &action.params {
            if let (ParamNeed::Defaulted(Value::Choice(default)), ParamType::Choice(options)) =
                (&param.need, &param.ty)
            {
                assert!(
                    options.iter().any(|o| &o.id == default),
                    "{} {}",
                    action.name,
                    param.name
                );
            }
        }
    }
}

#[test]
fn the_window_actions_declare_the_effects_the_model_charges() {
    use prov::Effect;
    let manifest = manifest().unwrap_or_else(|e| panic!("{e}"));
    for action in &manifest.manifest().actions {
        let name = action.name.as_str();
        if !(name.starts_with("shell.window.")
            || name.starts_with("shell.app.")
            || name == "shell.workspace.list")
        {
            continue;
        }
        let given = action
            .params
            .iter()
            .map(|p| (p.name.clone(), value_for(p)))
            .collect();
        let call = invocation(&action.name, given);
        let ask = window_ask_of(&call)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"))
            .unwrap_or_else(|| panic!("{name} is not a window ask"));
        assert_eq!(action.effect, ask.effect(), "{name}");
        let wants = if name == "shell.window.close" {
            Effect::Destructive
        } else {
            Effect::Read
        };
        assert_eq!(action.effect, wants, "{name}");
    }
}

#[test]
fn the_menu_activation_is_a_destructive_ceiling_the_provider_classifies_under() {
    use docket_core::PerCall;
    use prov::Effect;
    let manifest = manifest().unwrap_or_else(|e| panic!("{e}"));
    let rows: Vec<_> = manifest
        .manifest()
        .actions
        .iter()
        .filter(|a| a.per_call == PerCall::Classified)
        .map(|a| (a.name.as_str().to_owned(), a.effect))
        .collect();
    assert_eq!(
        rows,
        vec![("shell.menu.activate".to_owned(), Effect::Destructive)],
        "only the menu activation classifies per call"
    );
}

#[test]
fn the_listings_are_declared_untrusted_text_and_the_screenshot_answers_with_a_path() {
    use docket_core::{ResultShape, TitleTrust};
    let manifest = manifest().unwrap_or_else(|e| panic!("{e}"));
    for action in &manifest.manifest().actions {
        let name = action.name.as_str();
        match (name, &action.result) {
            ("shell.window.list" | "shell.workspace.list", ResultShape::Value { trust, .. }) => {
                assert!(matches!(trust, TitleTrust::ThirdParty(_)), "{name}");
            }
            ("shell.window.list" | "shell.workspace.list" | "shell.screenshot", other) => {
                assert!(
                    matches!(other, ResultShape::Value { .. }),
                    "{name}: {other:?}"
                );
            }
            _ => {}
        }
    }
}
