//! The daemon's half of the colour picker's eyedropper: while the capture service holds a colour
//! question, ask the compositor for a still picture of each screen, hold the pictures on the link
//! for the eyedropper surfaces to draw ([`apply_note`] takes each picture in), and tell the
//! service an eyedropper exists while a surface does. [`spawn_follower`] follows the service and
//! the link; a question nobody can point into (no picture could be made) is cancelled.

use shell_host::HostHandle;
use sill_model::capture_pick::{
    CaptureCommand, CapturePicker, ColorOutcome, EyedropIn, EyedropOut, Eyedropper,
};
use sill_model::desktop::{DesktopCommand, OutputPicture, ShellNote};
use sill_services::Control;
use sill_ui_kit::links::{EyedropperLink, Links};
use sill_ui_kit::services::Services;
use tokio::runtime::Handle;

use crate::daemon::msg::ShellMsg;

/// The colour question `picker` holds, and the screens to take a picture of, as the link's input.
///
/// ```
/// use sill::eyedropper::asked_of;
/// use sill_model::capture_pick::{CapturePicker, EyedropIn};
///
/// assert_eq!(asked_of(&CapturePicker::default()), EyedropIn::Asked(None));
/// ```
pub fn asked_of(picker: &CapturePicker) -> EyedropIn {
    EyedropIn::Asked(picker.color.as_ref().map(|asking| {
        let screens = picker.candidates.outputs.iter();
        (
            asking.id,
            screens.map(|screen| screen.name.clone()).collect(),
        )
    }))
}

/// Whether an eyedropper is drawn, given how many surfaces exist.
pub fn presence_of(surfaces: u32) -> Eyedropper {
    match surfaces {
        0 => Eyedropper::Absent,
        _ => Eyedropper::Present,
    }
}

/// The request for the compositor that `out` asks for, if it asks for one.
pub fn desktop_command(out: &EyedropOut) -> Option<DesktopCommand> {
    match out {
        EyedropOut::Capture(output) => Some(DesktopCommand::CaptureOutput {
            output: output.clone(),
        }),
        EyedropOut::Cancel(_) => None,
    }
}

/// The command for the capture service that `out` asks for, if it asks for one.
pub fn capture_command(out: &EyedropOut) -> Option<CaptureCommand> {
    match out {
        EyedropOut::Cancel(id) => Some(CaptureCommand::ColorAnswer(*id, ColorOutcome::Cancel)),
        EyedropOut::Capture(_) => None,
    }
}

/// Take the compositor's answer to a request for the picture of `output` into the link, and end
/// the question when no picture came of any screen.
pub fn take_picture(
    link: &EyedropperLink,
    capture: &Control<CaptureCommand>,
    output: String,
    picture: OutputPicture,
) {
    for out in link.take(EyedropIn::Picture(output, picture)) {
        if let Some(command) = capture_command(&out) {
            // The service may be gone; nobody is left to ask the person then.
            let _ = capture.send(command);
        }
    }
}

/// What a note of the compositor's shell protocol does to the eyedropper: a picture of a screen
/// is taken in; every other note is not its.
pub fn apply_note(link: &EyedropperLink, capture: &Control<CaptureCommand>, note: ShellNote) {
    if let ShellNote::OutputPicture { output, picture } = note {
        take_picture(link, capture, output, picture);
    }
}

/// `command` could not be carried (no compositor of ours runs): a request for a picture is
/// answered as failed, so the question is not left waiting.
pub fn refuse(link: &EyedropperLink, capture: &Control<CaptureCommand>, command: &DesktopCommand) {
    if let DesktopCommand::CaptureOutput { output } = command {
        take_picture(link, capture, output.clone(), OutputPicture::Failed);
    }
}

/// Follow the capture service and the link on `rt` until the host or the service is gone: a
/// colour question is put to the link, which says which pictures to ask the host's desktop arm
/// for, and the service is told whether an eyedropper surface exists.
pub fn spawn_follower(rt: &Handle, links: &Links, services: &Services, host: HostHandle<ShellMsg>) {
    let link = links.eyedropper.clone();
    let capture = services.capture.control.clone();
    let mut picker = services.capture.state.clone();
    let mut feed = link.subscribe();
    rt.spawn(async move {
        let mut told = Eyedropper::Absent;
        loop {
            let asked = asked_of(&picker.borrow_and_update());
            for out in link.take(asked) {
                if let Some(command) = desktop_command(&out)
                    && host.send(ShellMsg::Command(command)).is_err()
                {
                    return;
                }
                if let Some(command) = capture_command(&out) {
                    let _ = capture.send(command);
                }
            }
            let present = presence_of(feed.borrow_and_update().surfaces);
            if present != told {
                told = present;
                let _ = capture.send(CaptureCommand::Eyedropper(present));
            }
            let ended = tokio::select! {
                changed = picker.changed() => changed.is_err(),
                changed = feed.changed() => changed.is_err(),
            };
            if ended {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use sill_model::capture_pick::{
        AskId, Candidates, CaptureCommand, CapturePicker, ColorAsk, ColorAsking, ColorOutcome,
        EyedropIn, EyedropOut, Eyedropper, OutputCandidate,
    };
    use sill_model::desktop::{DesktopCommand, OutputPicture, ShellNote};
    use sill_services::Control;
    use sill_ui_kit::links::EyedropperLink;

    use super::{apply_note, asked_of, presence_of, refuse};

    fn picker_asking() -> CapturePicker {
        let screen = |name: &str| OutputCandidate {
            name: name.to_owned(),
            title: name.to_owned(),
        };
        CapturePicker {
            candidates: Candidates {
                outputs: vec![screen("DP-1"), screen("eDP-1")],
                windows: Vec::new(),
            },
            color: Some(ColorAsking {
                id: AskId(4),
                ask: ColorAsk {
                    app_label: "Notes".to_owned(),
                },
            }),
            ..CapturePicker::default()
        }
    }

    #[test]
    fn a_colour_question_names_every_screen_and_no_question_names_none() {
        assert_eq!(
            asked_of(&picker_asking()),
            EyedropIn::Asked(Some((
                AskId(4),
                vec!["DP-1".to_owned(), "eDP-1".to_owned()]
            )))
        );
        let sharing_only = CapturePicker {
            color: None,
            ..picker_asking()
        };
        assert_eq!(asked_of(&sharing_only), EyedropIn::Asked(None));
        assert_eq!(presence_of(0), Eyedropper::Absent);
        assert_eq!(presence_of(2), Eyedropper::Present);
    }

    #[test]
    fn the_last_failed_picture_cancels_the_question_and_a_picture_of_another_note_does_nothing() {
        let (capture, mut sent) = Control::<CaptureCommand>::channel();
        let link = EyedropperLink::new();
        let outs = link.take(asked_of(&picker_asking()));
        assert_eq!(
            outs,
            vec![
                EyedropOut::Capture("DP-1".to_owned()),
                EyedropOut::Capture("eDP-1".to_owned())
            ]
        );
        let failed = |output: &str| ShellNote::OutputPicture {
            output: output.to_owned(),
            picture: OutputPicture::Failed,
        };
        apply_note(&link, &capture, failed("DP-1"));
        apply_note(&link, &capture, ShellNote::Killed);
        assert!(sent.try_recv().is_err(), "one screen is still to come");
        apply_note(&link, &capture, failed("eDP-1"));
        assert_eq!(
            sent.try_recv().ok(),
            Some(CaptureCommand::ColorAnswer(AskId(4), ColorOutcome::Cancel))
        );
        assert!(sent.try_recv().is_err(), "once");
    }

    #[test]
    fn a_request_nobody_carried_is_answered_as_failed() {
        let (capture, mut sent) = Control::<CaptureCommand>::channel();
        let link = EyedropperLink::new();
        let _asked = link.take(asked_of(&picker_asking()));
        refuse(&link, &capture, &DesktopCommand::CancelSplit);
        assert!(sent.try_recv().is_err(), "another command is not a picture");
        for screen in ["DP-1", "eDP-1"] {
            refuse(
                &link,
                &capture,
                &DesktopCommand::CaptureOutput {
                    output: screen.to_owned(),
                },
            );
        }
        assert_eq!(
            sent.try_recv().ok(),
            Some(CaptureCommand::ColorAnswer(AskId(4), ColorOutcome::Cancel))
        );
    }
}
