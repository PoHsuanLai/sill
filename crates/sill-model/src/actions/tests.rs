use ds_core::word::Word;
use sill_launcher::actions::ActionId;
use sill_launcher::{AppId, Confirm, DesktopAction, SessionAction, SettingsLink, WindowKey};

use super::{Ask, Effect, Needs, Panel, Start, Subject, def, panel_action, screenshot_action};
use crate::screenshots::ShotKind;

#[test]
fn every_action_has_a_label_an_effect_and_a_chord_where_one_stands_for_it() {
    for &id in ActionId::ALL {
        let action = def(id);
        assert_eq!(action.id, id);
        assert!(!action.label.trim().is_empty(), "{id:?}");
        assert_eq!(action.confirm, action.effect.confirm(), "{id:?}");
    }
    let chords: Vec<ActionId> = ActionId::ALL
        .iter()
        .copied()
        .filter(|id| def(*id).chord.is_some())
        .collect();
    let want = [
        ActionId::Lock,
        ActionId::LogOut,
        ActionId::Launcher,
        ActionId::EmojiAndSymbols,
        ActionId::ForceQuitPanel,
        ActionId::ScreenshotArea,
        ActionId::ScreenshotScreen,
        ActionId::MissionControl,
        ActionId::NextSpace,
        ActionId::PreviousSpace,
        ActionId::NextWindow,
        ActionId::SwitcherNext,
        ActionId::SwitcherPrev,
        ActionId::OpenSettings,
        ActionId::HideApp,
        ActionId::QuitApp,
        ActionId::ToggleDockHide,
    ];
    assert_eq!(chords, want);
}

#[test]
fn no_two_actions_share_a_label_or_a_chord() {
    for (at, id) in ActionId::ALL.iter().enumerate() {
        for other in &ActionId::ALL[..at] {
            assert_ne!(def(*id).label, def(*other).label, "{id:?} {other:?}");
            if let Some(chord) = def(*id).chord {
                assert_ne!(Some(chord), def(*other).chord, "{id:?} {other:?}");
            }
        }
    }
}

#[test]
fn the_session_actions_ask_as_the_power_menu_does() {
    for action in SessionAction::ALL {
        let id = ActionId::of_session(action);
        assert_eq!(def(id).effect, Effect::Session(action), "{action:?}");
        assert_eq!(def(id).confirm, action.confirm(), "{action:?}");
    }
    assert_eq!(def(ActionId::Lock).confirm, Confirm::Now);
    assert_eq!(def(ActionId::ShutDown).confirm, Confirm::Ask);
}

#[test]
fn a_screenshot_kind_and_a_panel_find_their_action() {
    for kind in [ShotKind::Area, ShotKind::Window, ShotKind::Screen] {
        assert_eq!(def(screenshot_action(kind)).effect, Effect::Shot(kind));
    }
    for panel in [
        Panel::Launcher,
        Panel::PowerMenu,
        Panel::ForceQuit,
        Panel::NotificationCenter,
        Panel::ControlCenter,
    ] {
        assert_eq!(def(panel_action(panel)).effect, Effect::Panel(panel));
    }
}

#[test]
fn an_action_is_refused_what_it_cannot_act_on() {
    let app = || Subject::App(AppId("term".to_owned()));
    let started = || {
        Subject::Start(Start {
            app: AppId("term".to_owned()),
            action: DesktopAction::Named("new-window".to_owned()),
            uris: Vec::new(),
        })
    };
    let windows = || Subject::Windows(vec![WindowKey(1)]);
    let place = || Subject::Settings(SettingsLink("page:wallpaper".to_owned()));
    let cases = [
        (ActionId::Lock, Subject::None, true),
        (ActionId::Lock, app(), false),
        (ActionId::OpenSettings, Subject::None, true),
        (ActionId::OpenSettings, place(), true),
        (ActionId::OpenSettings, app(), false),
        (ActionId::OpenApp, app(), true),
        (ActionId::OpenApp, started(), true),
        (ActionId::OpenApp, Subject::None, false),
        (ActionId::PinApp, app(), true),
        (ActionId::PinApp, Subject::None, false),
        (ActionId::HideApp, windows(), true),
        (ActionId::HideApp, app(), false),
        (ActionId::QuitApp, windows(), true),
        (ActionId::ForceQuitApp, app(), true),
        (ActionId::ForceQuitApp, windows(), false),
    ];
    for (id, subject, fits) in cases {
        let ask = Ask::on(id, subject);
        assert_eq!(ask.refusal().is_none(), fits, "{ask:?}");
    }
    assert_eq!(
        Ask::on(ActionId::PinApp, Subject::None)
            .refusal()
            .as_deref(),
        Some("Keep in Dock needs an app.")
    );
    assert_eq!(def(ActionId::Lock).effect.needs(), Needs::Nothing);
}

#[test]
fn an_ask_has_a_pinned_wire_form() {
    let cases = [
        (
            Ask::of(ActionId::ShowDesktop),
            r#"{"action":"show-desktop","subject":{"kind":"none"}}"#,
        ),
        (
            Ask::on(ActionId::PinApp, Subject::App(AppId("term".to_owned()))),
            r#"{"action":"pin-app","subject":{"kind":"app","v":"term"}}"#,
        ),
        (
            Ask::on(
                ActionId::OpenApp,
                Subject::Start(Start {
                    app: AppId("term".to_owned()),
                    action: DesktopAction::Named("new-window".to_owned()),
                    uris: vec!["trash:///".to_owned()],
                }),
            ),
            r#"{"action":"open-app","subject":{"kind":"start","v":{"app":"term","action":{"kind":"named","v":"new-window"},"uris":["trash:///"]}}}"#,
        ),
        (
            Ask::on(
                ActionId::QuitApp,
                Subject::Windows(vec![WindowKey(1), WindowKey(2)]),
            ),
            r#"{"action":"quit-app","subject":{"kind":"windows","v":[1,2]}}"#,
        ),
        (
            Ask::on(
                ActionId::OpenSettings,
                Subject::Settings(SettingsLink("page:wallpaper".to_owned())),
            ),
            r#"{"action":"open-settings","subject":{"kind":"settings","v":"page:wallpaper"}}"#,
        ),
    ];
    for (ask, wire) in cases {
        assert_eq!(serde_json::to_string(&ask).unwrap_or_default(), wire);
        let back: Result<Ask, _> = serde_json::from_str(wire);
        assert_eq!(back.ok(), Some(ask));
    }
}

#[test]
fn every_action_is_written_as_its_slug_on_the_wire() {
    for &id in ActionId::ALL {
        let wire = serde_json::to_string(&id).unwrap_or_default();
        assert_eq!(wire, format!("\"{}\"", id.slug()), "{id:?}");
    }
}

#[test]
fn only_the_power_menu_opens_where_the_rest_flip() {
    use super::Way;
    for panel in [
        Panel::Launcher,
        Panel::ForceQuit,
        Panel::NotificationCenter,
        Panel::ControlCenter,
    ] {
        assert_eq!(panel.way(), Way::Toggle, "{panel:?}");
    }
    assert_eq!(Panel::PowerMenu.way(), Way::Open);
}

#[test]
fn no_two_actions_share_a_slug() {
    for (at, id) in ActionId::ALL.iter().enumerate() {
        for other in &ActionId::ALL[..at] {
            assert_ne!(id.slug(), other.slug(), "{id:?} {other:?}");
        }
    }
}
