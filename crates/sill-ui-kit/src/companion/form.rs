//! A form for the parameters an action still needs.

use companion_wire::FormWire;
use docket_core::{Args, ParamDecl, ParamNeed, ParamType, Value};
use ds::components::companion::answer::action::{ActionRole, CardAction, CardActionId};
use ds::components::companion::answer::footer::CardFooter;
use ds::components::companion::form::model::{
    CompactForm, EntityOption, FieldKeyId, FieldNeed, FormField, FormInput, FormValue,
};
use ds::components::fields::text_field_model::Validity;
use ds_core::vocab::{Availability, EffectMark};

/// The form for `form`, titled `title`; its one button submits it as `submit`.
pub fn form_view(form: &FormWire, title: &str, submit: &str, footer: CardFooter) -> CompactForm {
    CompactForm {
        title: title.to_owned(),
        fields: form
            .params
            .iter()
            .map(|param| field(param, &form.values))
            .collect(),
        submit: CardAction {
            id: CardActionId("submit".to_owned()),
            label: submit.to_owned(),
            role: ActionRole::Primary,
            effect: EffectMark::UndoableWrite,
            keys: None,
            availability: Availability::Enabled,
        },
        footer,
    }
}

fn field(param: &ParamDecl, values: &Args) -> FormField {
    FormField {
        key: FieldKeyId(param.name.as_str().to_owned()),
        label: param.label.as_str().to_owned(),
        input: input(&param.ty),
        need: match param.need {
            ParamNeed::Required => FieldNeed::Required,
            ParamNeed::Optional | ParamNeed::Defaulted(_) => FieldNeed::Optional,
        },
        value: values
            .get(&param.name)
            .map_or(FormValue::Empty, |given| value(&given.value)),
        validity: Validity::Valid,
    }
}

fn input(ty: &ParamType) -> FormInput {
    match ty {
        ParamType::Text { lines, .. } => match lines {
            docket_core::Lines::One => FormInput::Text,
            docket_core::Lines::Many => FormInput::Lines,
        },
        ParamType::Integer { .. } | ParamType::Decimal { .. } | ParamType::Duration => {
            FormInput::Number
        }
        ParamType::Date => FormInput::Date,
        ParamType::DateTime => FormInput::Time,
        ParamType::Choice(choices) => FormInput::Choice(
            choices
                .iter()
                .map(|choice| {
                    (
                        choice.id.as_str().to_owned(),
                        choice.label.as_str().to_owned(),
                    )
                })
                .collect(),
        ),
        ParamType::Entity(_) | ParamType::Entities(_) => FormInput::Entity {
            suggestions: Vec::<EntityOption>::new(),
        },
        ParamType::File | ParamType::Url | ParamType::Dynamic(_) => FormInput::Text,
    }
}

/// What the form holds for a value already given.
fn value(given: &Value) -> FormValue {
    match given {
        Value::Text(text) | Value::Url(text) => FormValue::Text(text.clone()),
        Value::Integer(n) => FormValue::Number(n.to_string()),
        Value::Decimal(d) => FormValue::Number(d.units.to_string()),
        Value::Duration(seconds) => FormValue::Number(seconds.0.to_string()),
        Value::Choice(id) => FormValue::Choice(id.as_str().to_owned()),
        Value::Entity(entity) => FormValue::Entity(entity.key.as_str().to_owned()),
        Value::Date(_)
        | Value::DateTime(_)
        | Value::Entities(_)
        | Value::File(_)
        | Value::Handle(_)
        | Value::List(_)
        | Value::Record(_) => FormValue::Empty,
    }
}
