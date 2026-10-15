//! Night Shift on scratch directories (casement's config folder and sill's state folder under a
//! temporary root, no bus): the switch writes casement's `[night_light]` and keeps the manual
//! turn-on's end, turning it off puts casement back, a turn-on that ran out while sill was away
//! is ended at start, and where sill warms the displays itself casement is kept off.

use std::time::Duration;

use sill_model::night_shift::{
    Backend, NightShift, NightShiftCommand, Reach, Schedule, Shown, shown,
};
use sill_services::env::Env;
use sill_services::night_shift::NightShiftService;
use sill_services::service::{Handle, spawn};
use sill_testkit::TempDir;

const SETTLE: Duration = Duration::from_secs(5);

fn files(scratch: &TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
    let env = Env::isolated(scratch.path());
    (
        env.dirs.config.join("casement").join("settings.toml"),
        env.dirs.sill_state().join("night-shift.json"),
    )
}

fn start(scratch: &TempDir) -> Handle<NightShiftService> {
    let env = Env::isolated(scratch.path());
    spawn::<NightShiftService>(&tokio::runtime::Handle::current(), &env, ())
}

async fn until(
    night_shift: &mut Handle<NightShiftService>,
    want: impl Fn(&NightShift) -> bool,
) -> NightShift {
    let arrived = tokio::time::timeout(SETTLE, night_shift.state.wait_for(|state| want(state)))
        .await
        .is_ok_and(|seen| seen.is_ok());
    let state = *night_shift.state.borrow();
    assert!(arrived, "timed out; last {state:?}");
    state
}

fn mode_in(settings: &std::path::Path) -> String {
    let text = std::fs::read_to_string(settings).unwrap_or_default();
    let table: toml::Table = toml::from_str(&text).unwrap_or_default();
    table
        .get("night_light")
        .and_then(|section| section.get("mode"))
        .and_then(toml::Value::as_str)
        .unwrap_or("")
        .to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_switch_warms_until_tomorrow_and_off_puts_casement_back() {
    let scratch = TempDir::new("night-shift-switch");
    let (settings, manual) = files(&scratch);
    let mut service = start(&scratch);
    let ready = until(&mut service, |s| s.reach == Reach::Ready).await;
    assert_eq!(shown(&ready), Shown::Off);

    let _ = service.control.send(NightShiftCommand::TurnOnUntilTomorrow);
    let on = until(&mut service, |s| s.manual.is_some()).await;
    assert_eq!(shown(&on), Shown::OnUntilTomorrow);
    assert_eq!(mode_in(&settings), "always");
    assert!(manual.exists(), "the end of the turn-on is kept");

    let _ = service.control.send(NightShiftCommand::TurnOff);
    let off = until(&mut service, |s| s.manual.is_none()).await;
    assert_eq!(off.light.schedule, Schedule::Off);
    assert_eq!(mode_in(&settings), "off");
    assert!(!manual.exists(), "and forgotten");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_turn_on_that_ran_out_while_sill_was_away_is_ended_at_start() {
    let scratch = TempDir::new("night-shift-stale");
    let (settings, manual) = files(&scratch);
    for file in [&settings, &manual] {
        std::fs::create_dir_all(file.parent().unwrap_or(scratch.path()))
            .unwrap_or_else(|e| panic!("{e}"));
    }
    std::fs::write(&settings, "[night_light]\nmode = \"always\"\n")
        .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(
        &manual,
        r#"{"until":1,"restore":{"mode":"scheduled","from":1320,"to":420,"kelvin":3400,"fade_minutes":30}}"#,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let mut service = start(&scratch);
    let back = until(&mut service, |s| {
        s.reach == Reach::Ready && s.manual.is_none()
    })
    .await;
    assert_eq!(back.light.schedule, Schedule::Scheduled);
    assert_eq!(mode_in(&settings), "scheduled");
    assert!(!manual.exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn where_sill_warms_the_displays_casement_is_kept_off() {
    let scratch = TempDir::new("night-shift-hardware");
    let (settings, _) = files(&scratch);
    std::fs::create_dir_all(settings.parent().unwrap_or(scratch.path()))
        .unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(&settings, "[night_light]\nmode = \"scheduled\"\n")
        .unwrap_or_else(|e| panic!("{e}"));
    let mut service = start(&scratch);
    until(&mut service, |s| s.reach == Reach::Ready).await;
    let _ = service
        .control
        .send(NightShiftCommand::Backend(Backend::Hardware));
    let quiet = until(&mut service, |s| s.backend == Backend::Hardware).await;
    assert_eq!(shown(&quiet), Shown::ByDisplay);
    assert_eq!(quiet.light.schedule, Schedule::Off);
    assert_eq!(mode_in(&settings), "off");
}
