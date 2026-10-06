//! What the accounts sheet draws and sends, on the harness over a fake accounts-sheet service: a
//! window of its own, centred, with a titlebar that carries the step's title (never an attached
//! sheet); every step accountd can show (provider list, the sign-in form with its typed password,
//! the browser wait, the device code, the review, working, failed) and the consent alert; the keys
//! (Return the default, Escape a dismissal) and what each press tells accountd, taken through the
//! pure step so the controlled values follow what is typed. Nothing here reaches a bus.
//! `SILL_SHOTS` names a directory the steps are rendered into (light and dark).

use std::time::Duration;

use ds::prelude::*;
use ds_harness::{Driver, Input, Query, Viewport};
use ds_settings::Environment;
use porter_core::SecretText;
use porter_core::sheet::{FieldKind, FieldValue, SheetInput, SheetView};
use shell_host::OutputId;
use sill_model::accounts::sheet::{
    AccountsSheetCommand, AccountsSheets, SheetEvent, SheetHandle, SheetLink, SheetOut, step,
};
use sill_overlays::AccountsSheet;
use sill_ui_kit::harness::{SurfaceHarness, fake};
use sill_ui_kit::hooks::PinnedAppearance;
use sill_ui_kit::links::{BannerLink, BannerOutput};

const VIEWPORT: Viewport = Viewport {
    width: 1280,
    height: 800,
    scale_percent: 100,
};

const PROVIDERS: &str = r#"{"kind":"providers","v":[
  {"id":"nextcloud","label":"Nextcloud","mark":"nextcloud"},
  {"id":"google","label":"Google","mark":"google"},
  {"id":"generic-imap","label":"Email (IMAP)","mark":"mail"}]}"#;
const FORM: &str = r#"{"kind":"sign_in","v":{"provider":"nextcloud","fields":[
  {"kind":"address","entry":"plain","presence":"required","prefill":null},
  {"kind":"password","entry":"secret","presence":"required","prefill":null}],"problem":null}}"#;
const REFUSED: &str = r#"{"kind":"sign_in","v":{"provider":"nextcloud","fields":[
  {"kind":"address","entry":"plain","presence":"required","prefill":null},
  {"kind":"password","entry":"secret","presence":"required","prefill":null}],
  "problem":{"field":"password","problem":"refused"}}}"#;
const BROWSER: &str = r#"{"kind":"browser_wait","v":{"provider":"google","url":"https://accounts.example.org/auth?c=1"}}"#;
const CODE: &str = r#"{"kind":"show_code","v":{"provider":"microsoft","user_code":"ABCD-EFGH","url":"https://example.org/device"}}"#;
const WORKING: &str = r#"{"kind":"working","v":"nextcloud"}"#;
const FAILED: &str = r#"{"kind":"failed","v":{"provider":"nextcloud","fault":"unreachable"}}"#;
const DONE: &str = r#"{"kind":"done"}"#;
const REVIEW: &str = r#"{"kind":"review","v":{"provider":"nextcloud","allow":{"name":"org.quire.Mail","isolation":"flatpak"}
  ,"review":{"label":"ada@example.org","endpoints":[],"services":[
  {"kind":"mail","state":{"kind":"offered","v":"on"},"limit":null},
  {"kind":"calendar","state":{"kind":"offered","v":"on"},"limit":null},
  {"kind":"photos","state":{"kind":"absent","v":"provider_offers_none"},"limit":null}]}}}"#;
const CONSENT: &str = r#"{"kind":"consent","v":{"app":{"name":"org.quire.Photos","isolation":"flatpak"},
  "kind":"storage","class":"app_own","usage":"interactive","accounts":[
  {"account":"home","label":"ada@home.example","provider":"nextcloud"},
  {"account":"work","label":"ada@work.example","provider":"microsoft"}]}}"#;

fn view(json: &str) -> SheetView {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}: {json}"))
}

fn handle() -> SheetHandle {
    SheetHandle::parse("accountd-0").unwrap_or_else(|| panic!("handle"))
}

/// The sheet on a running harness, with the service's state kept here and stepped as the service
/// would, so what is typed comes back as props.
struct Run {
    running: SurfaceHarness<AccountsSheet>,
    sheets: AccountsSheets,
    /// What accountd would have been told, in order.
    told: Vec<SheetInput>,
    /// The pages the sheet asked to open.
    pages: usize,
}

fn start_on(output: OutputId, environment: Environment) -> Run {
    let link = BannerLink::new();
    link.set(BannerOutput::On(output));
    let mut running = SurfaceHarness::<AccountsSheet>::setup(link, ())
        .viewport(VIEWPORT)
        .appearance(PinnedAppearance(environment))
        .start();
    running.harness.advance(Duration::from_millis(40));
    let sheets = AccountsSheets {
        link: SheetLink::Serving,
        ..AccountsSheets::default()
    };
    running.feeds.accounts_sheet.send_replace(sheets.clone());
    Run {
        running,
        sheets,
        told: Vec::new(),
        pages: 0,
    }
}

fn start() -> Run {
    start_on(fake::output().id, Environment::default())
}

impl Run {
    fn feed(&mut self, event: SheetEvent) {
        let (next, out) = step(std::mem::take(&mut self.sheets), event);
        self.sheets = next;
        for effect in out {
            match effect {
                SheetOut::Reply(_, input) => self.told.push(input),
                SheetOut::OpenPage(_) => self.pages += 1,
            }
        }
        self.running
            .feeds
            .accounts_sheet
            .send_replace(self.sheets.clone());
        self.running.harness.advance(Duration::from_millis(60));
    }

    fn open(&mut self, json: &str) {
        self.feed(SheetEvent::Opened(handle(), view(json)));
    }

    fn update(&mut self, json: &str) {
        self.feed(SheetEvent::Updated(handle(), view(json)));
    }

    /// Carry what the drawn parts sent to the service into the stepped state.
    fn pump(&mut self) {
        while let Ok(command) = self.running.feeds.accounts_sheet_commands.try_recv() {
            if let AccountsSheetCommand::Act(handle, action) = command {
                self.feed(SheetEvent::Acted(handle, action));
            }
        }
    }

    fn key(&mut self, key: ShortcutKey) {
        self.running.harness.send(Input::key(key));
        self.running.harness.advance(Duration::from_millis(40));
        self.pump();
    }

    fn text(&mut self, text: &str) {
        for c in text.chars() {
            self.key(ShortcutKey::Char(c));
        }
    }

    fn click(&mut self, selector: &str) {
        let at = self.running.harness.centre(selector).unwrap_or_else(|| {
            panic!(
                "{selector} is not laid out: {}",
                self.running.harness.html()
            )
        });
        self.running.harness.send(Input::click(at));
        self.running.harness.advance(Duration::from_millis(40));
        self.pump();
    }

    /// The `nth` (from 1) button of the step's row of buttons (`ds-acc-actions`), or of the
    /// alert's stack (`ds-alert-slot`).
    fn press(&mut self, nth: usize) {
        let row = if self.count(".ds-alert") == 1 {
            format!(".ds-alert-footer span:nth-of-type({nth}) .ds-button")
        } else {
            format!(".ds-acc-actions button:nth-of-type({nth})")
        };
        self.click(&row);
    }

    /// How many pages the sheet has asked to open since the last call.
    fn opened(&mut self) -> usize {
        std::mem::take(&mut self.pages)
    }

    fn told(&mut self) -> Vec<SheetInput> {
        std::mem::take(&mut self.told)
    }

    fn count(&self, selector: &str) -> usize {
        self.running.harness.count(selector)
    }
}

#[test]
fn with_no_sheet_it_draws_nothing() {
    let mut run = start();
    run.running.harness.advance(Duration::from_millis(100));
    assert_eq!(run.count(".sill-accounts-window"), 0);
    assert_eq!(run.count(".ds-sheet"), 0);
}

#[test]
fn a_sheet_for_another_output_draws_nothing_here() {
    let mut run = start_on(OutputId(99), Environment::default());
    run.open(PROVIDERS);
    assert_eq!(run.count(".sill-accounts-window"), 0);
}

#[test]
fn a_finished_add_draws_nothing() {
    let mut run = start();
    run.open(DONE);
    assert_eq!(run.count(".sill-accounts-window"), 0);
}

#[test]
fn every_step_is_a_centred_window_with_its_title_in_a_titlebar_and_never_an_attached_sheet() {
    let steps = [
        (PROVIDERS, "Add Account"),
        (FORM, "Sign in to Nextcloud"),
        (BROWSER, "Continue in your browser"),
        (CODE, "Enter this code"),
        (REVIEW, "Choose what to use"),
        (WORKING, "Signing in to Nextcloud"),
        (FAILED, "Could not add Nextcloud"),
        (CONSENT, "Account Access"),
    ];
    for (json, title) in steps {
        let mut run = start();
        run.open(json);
        let h = &run.running.harness;
        assert_eq!(h.count(".sill-accounts-window"), 1, "{title}");
        assert_eq!(h.count(".ds-sheet"), 0, "{title}: an attached sheet");
        assert_eq!(h.count(".ds-titlebar"), 1, "{title}");
        assert_eq!(
            h.count(".ds-acc-title"),
            0,
            "{title}: the step draws no title of its own"
        );
        assert_eq!(
            h.text_of(".ds-titlebar").as_deref().map(str::trim),
            Some(title)
        );
        assert_eq!(
            h.count(".ds-traffic-lights"),
            0,
            "{title}: nobody closes a dialog by a light"
        );
        let window = h.rect(".sill-accounts-window").expect("laid out"); // test-only
        let middle = (
            window.origin.x.0 + window.size.width.0 / 2.0,
            window.origin.y.0 + window.size.height.0 / 2.0,
        );
        assert!(
            (middle.0 - 640.0).abs() < 1.0,
            "{title}: centred across {window:?}"
        );
        assert!(
            (middle.1 - 400.0).abs() < 1.0,
            "{title}: centred down {window:?}"
        );
        assert!(window.size.width.0 <= 560.5, "{title}: {window:?}");
    }
}

#[test]
fn the_add_account_password_flow_sends_what_was_typed_and_keeps_no_secret() {
    let mut run = start();
    run.open(PROVIDERS);
    run.click(".ds-list .ds-row");
    let told = run.told();
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(
        matches!(&told[0], SheetInput::Pick(id) if id.as_str() == "nextcloud"),
        "{told:?}"
    );

    run.update(FORM);
    run.text("ada@example.org");
    run.key(ShortcutKey::Tab);
    run.text("hunter2");
    assert!(
        !run.running.harness.html().contains("hunter2"),
        "the password is not in the markup"
    );
    run.key(ShortcutKey::Enter);
    let told = run.told();
    let [SheetInput::Submit(answers)] = told.as_slice() else {
        panic!("one submit: {told:?}");
    };
    assert_eq!(answers[0].kind, FieldKind::Address);
    assert_eq!(
        answers[0].value,
        FieldValue::Plain("ada@example.org".to_owned())
    );
    assert_eq!(
        answers[1].value,
        FieldValue::Secret(SecretText::new("hunter2"))
    );
    assert!(
        !format!("{:?}", run.sheets).contains("hunter2"),
        "the secret left the draft when it was sent"
    );
    // The form comes back refused: the address stays, the password has to be typed again.
    run.update(REFUSED);
    assert_eq!(run.count(".ds-acc-step[data-step=sign-in]"), 1);
    run.key(ShortcutKey::Enter);
    assert_eq!(
        run.told(),
        vec![],
        "Continue stays disabled with the password empty"
    );
}

#[test]
fn failed_then_try_again_and_back_and_cancel_say_so() {
    let mut run = start();
    run.open(FAILED);
    run.key(ShortcutKey::Enter);
    assert_eq!(run.told(), vec![SheetInput::Retry]);
    run.update(FAILED);
    run.press(1);
    assert_eq!(run.told(), vec![SheetInput::Back]);
    run.update(FAILED);
    run.press(2);
    assert_eq!(run.told(), vec![SheetInput::Dismiss]);
    run.key(ShortcutKey::Escape);
    assert_eq!(run.told(), vec![SheetInput::Dismiss]);
}

#[test]
fn escape_dismisses_at_every_step() {
    for json in [PROVIDERS, FORM, BROWSER, CODE, REVIEW, WORKING, FAILED] {
        let mut run = start();
        run.open(json);
        run.key(ShortcutKey::Escape);
        assert_eq!(run.told(), vec![SheetInput::Dismiss], "{json}");
    }
}

#[test]
fn the_review_confirms_with_its_switches() {
    let mut run = start();
    run.open(REVIEW);
    run.key(ShortcutKey::Enter);
    let told = run.told();
    let [SheetInput::Confirm(choices)] = told.as_slice() else {
        panic!("one confirm: {told:?}");
    };
    assert_eq!(
        choices.len(),
        2,
        "the absent service has no switch: {choices:?}"
    );
    assert!(
        run.running
            .harness
            .html()
            .contains("Add, and allow Mail to use it")
    );
}

#[test]
fn open_again_asks_accountd_to_show_the_page_again_and_the_repeat_opens_it() {
    let mut run = start();
    run.open(BROWSER);
    assert_eq!(run.opened(), 1, "shown the page, the sheet opens it");
    run.key(ShortcutKey::Enter);
    assert_eq!(run.told(), vec![SheetInput::OpenAgain]);
    // accountd answers by showing the same page again.
    run.update(BROWSER);
    assert_eq!(run.opened(), 1, "the repeat opens it again");
    run.update(WORKING);
    assert_eq!(run.opened(), 0);
}

#[test]
fn the_consent_alert_answers_once_always_deny_and_escape_dismisses() {
    use porter_core::consent::{ConsentAnswer, GrantScope};
    let allow = |scope| {
        SheetInput::Answer(ConsentAnswer::Allow {
            account: porter_core::AccountId::parse("home").unwrap_or_else(|e| panic!("{e}")),
            scope,
        })
    };
    let mut run = start();
    run.open(CONSENT);
    assert_eq!(run.count(".ds-alert"), 1);
    run.key(ShortcutKey::Enter);
    assert_eq!(run.told(), vec![allow(GrantScope::Once)]);
    run.update(CONSENT);
    run.press(2);
    assert_eq!(run.told(), vec![allow(GrantScope::Always)]);
    run.update(CONSENT);
    run.press(3);
    assert_eq!(run.told(), vec![SheetInput::Answer(ConsentAnswer::Deny)]);
    run.update(CONSENT);
    run.key(ShortcutKey::Escape);
    assert_eq!(run.told(), vec![SheetInput::Dismiss]);
}

#[test]
fn add_account_on_an_alert_with_no_fitting_account_answers_add_account() {
    use porter_core::consent::ConsentAnswer;
    let mut run = start();
    run.open(
        r#"{"kind":"consent","v":{"app":{"name":"org.quire.Photos","isolation":"flatpak"},
        "kind":"storage","class":"app_own","usage":"interactive","accounts":[]}}"#,
    );
    run.key(ShortcutKey::Enter);
    assert_eq!(
        run.told(),
        vec![SheetInput::Answer(ConsentAnswer::AddAccount)]
    );
}

/// The steps as pictures, light and dark, when `SILL_SHOTS` names a directory.
#[test]
fn the_steps_render() {
    let Some(dir) = std::env::var_os("SILL_SHOTS") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}")); // test-only
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut environment = Environment::default();
        environment.settings.appearance.theme = theme;
        for (json, step_name) in [
            (PROVIDERS, "providers"),
            (FORM, "sign-in"),
            (REFUSED, "sign-in-refused"),
            (BROWSER, "browser"),
            (CODE, "code"),
            (REVIEW, "review"),
            (WORKING, "working"),
            (FAILED, "failed"),
            (CONSENT, "consent"),
        ] {
            let mut run = start_on(fake::output().id, environment.clone());
            run.open(json);
            if json == FORM {
                run.text("ada@example.org");
            }
            run.running.harness.advance(Duration::from_millis(600));
            let image = run.running.harness.render().expect("renders"); // test-only
            image
                .save(dir.join(format!("accounts-sheet-{step_name}-{name}.png")))
                .unwrap_or_else(|e| panic!("{e}")); // test-only
        }
    }
}
