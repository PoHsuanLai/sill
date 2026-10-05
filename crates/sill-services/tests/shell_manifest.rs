//! The shell's manifest (`dist/intents/org.quire.Shell.toml`) against the model that reads an
//! invocation of it: every declared action is one the model knows, every choice a declared
//! option, so a name or an option cannot drift between the file docket reads and the code sill
//! runs.

use std::collections::BTreeMap;

use docket_core::{
    AppRefusal, CallId, ChoiceId, Invocation, Origin, ParamDecl, ParamNeed, ParamType, Value,
};
use prov::{ActionName, Actor, Label, Labelled, SpaceId};
use sill_model::intents::shell::{action_of, menu_ask_of};
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
        other => panic!("{}: the shell declares no {other:?} parameter", param.name),
    }
}

/// An invocation read as the shell reads it: a menu ask, or one of its other actions.
fn reads(invocation: &Invocation) -> Result<(), AppRefusal> {
    match menu_ask_of(invocation)? {
        Some(_) => Ok(()),
        None => action_of(invocation).map(drop),
    }
}

#[test]
fn the_shipped_manifest_is_valid_and_is_the_shells() {
    let manifest = manifest().unwrap_or_else(|e| panic!("{e}"));
    let manifest = manifest.manifest();
    assert_eq!(manifest.app.as_str(), "org.quire.Shell");
    assert_eq!(manifest.actions.len(), 8);
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
