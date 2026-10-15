//! The first start offers the model picker once: the first-run page opens and the offer is
//! recorded; a later start, or a start after the record was written, opens nothing. Scratch
//! directories only, no bus.

use sill_model::launch::{LaunchCommand, SettingsPage};
use sill_model::seed::ModelOffered;
use sill_services::control::Control;
use sill_services::env::Dirs;
use sill_services::model_offer::{offer_first_run, path_in};
use sill_testkit::TempDir;

fn opened(rx: &mut tokio::sync::mpsc::UnboundedReceiver<LaunchCommand>) -> Vec<LaunchCommand> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

#[test]
fn the_first_start_opens_the_first_run_picker_and_a_later_one_does_not() {
    let scratch = TempDir::new("model-offer");
    let dirs = Dirs::under(scratch.path());
    let (launch, mut heard) = Control::channel();

    assert!(!path_in(&dirs).exists(), "a fresh account has no record");
    offer_first_run(&dirs, &launch).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        opened(&mut heard),
        vec![LaunchCommand::OpenSettingsPage(
            SettingsPage::AssistantModelSetup
        )]
    );
    let text = std::fs::read_to_string(path_in(&dirs)).unwrap_or_else(|e| panic!("{e}"));
    let record: ModelOffered = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(record.page, SettingsPage::AssistantModelSetup);

    offer_first_run(&dirs, &launch).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(opened(&mut heard), Vec::new(), "the second start is quiet");
}

#[test]
fn a_record_that_is_already_there_means_no_offer() {
    let scratch = TempDir::new("model-offer-recorded");
    let dirs = Dirs::under(scratch.path());
    let path = path_in(&dirs);
    std::fs::create_dir_all(path.parent().unwrap_or_else(|| panic!("a folder")))
        .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(&path, "not even json").unwrap_or_else(|e| panic!("{e}"));
    let (launch, mut heard) = Control::channel();

    offer_first_run(&dirs, &launch).unwrap_or_else(|e| panic!("{e}"));

    assert_eq!(opened(&mut heard), Vec::new());
}
