//! Automatic for one helper: the person ticks the models, nothing is ticked or chosen for them,
//! and the pool goes out only when they confirm.

use super::*;

fn qwen() -> ModelRef {
    ModelRef::parse("local/qwen3-4b").unwrap_or_else(|| panic!("model"))
}

fn claude() -> ModelRef {
    ModelRef::parse("cloud/claude-sonnet").unwrap_or_else(|| panic!("model"))
}

#[test]
fn choosing_automatic_ticks_nothing_and_sets_no_model() {
    let (model, sent) = update(
        opened("h-1", HelperModel::Unset),
        HelpersMsg::StartAutomatic,
    );
    assert!(sent.is_empty());
    assert_eq!(model.automatic, Automatic::Choosing(BTreeSet::new()));
    let draft = model.draft().unwrap_or_else(|| panic!("a card is open"));
    assert_eq!(draft.model, HelperModel::Unset);
}

#[test]
fn a_helper_not_made_yet_has_no_pool_to_choose() {
    let (model, _) = update(HelpersModel::default(), HelpersMsg::NewHelper);
    let (model, _) = update(model, HelpersMsg::StartAutomatic);
    assert_eq!(model.automatic, Automatic::Off);
}

#[test]
fn ticks_toggle_and_confirming_sends_the_pool_for_that_helper_only() {
    let (model, sent) = feed(
        opened("h-1", HelperModel::Unset),
        vec![
            HelpersMsg::StartAutomatic,
            HelpersMsg::TogglePooled(qwen()),
            HelpersMsg::TogglePooled(claude()),
            HelpersMsg::TogglePooled(qwen()),
            HelpersMsg::ConfirmAutomatic,
        ],
    );
    assert_eq!(
        sent,
        [HelpersCommand::SetPool {
            helper: id("h-1"),
            pool: [claude()].into_iter().collect()
        }]
    );
    let (model, _) = update(model, HelpersMsg::PoolWritten(true));
    assert_eq!(model.automatic, Automatic::Off);
    assert_eq!(
        model.notice.as_deref(),
        Some("Automatic is on for this helper.")
    );
}

#[test]
fn confirming_with_nothing_ticked_says_so_and_sends_nothing() {
    let (model, sent) = feed(
        opened("h-1", HelperModel::Unset),
        vec![HelpersMsg::StartAutomatic, HelpersMsg::ConfirmAutomatic],
    );
    assert!(sent.is_empty());
    assert_eq!(
        model.notice.as_deref(),
        Some("Tick at least one model for Automatic to use.")
    );
}

#[test]
fn a_pool_that_could_not_be_kept_is_said_and_cancel_and_closing_the_card_give_it_up() {
    let (model, _) = update(
        update(
            opened("h-1", HelperModel::Unset),
            HelpersMsg::StartAutomatic,
        )
        .0,
        HelpersMsg::PoolWritten(false),
    );
    assert_eq!(
        model.notice.as_deref(),
        Some("Automatic could not be set just now.")
    );
    let (started, _) = update(
        opened("h-1", HelperModel::Unset),
        HelpersMsg::StartAutomatic,
    );
    let (cancelled, _) = update(started.clone(), HelpersMsg::CancelAutomatic);
    assert_eq!(cancelled.automatic, Automatic::Off);
    let (closed, _) = update(started, HelpersMsg::CloseCard);
    assert_eq!(closed.automatic, Automatic::Off);
}
