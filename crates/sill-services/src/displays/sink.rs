//! Where an output's scale is read and set: the [`ScaleSink`] seam, and its one real side,
//! [`CosmicRandr`], which runs the `cosmic-randr` tool COSMIC ships. cosmic-comp persists what
//! it applies in `$XDG_STATE_HOME/cosmic-comp/outputs.ron` itself, so a scale set here survives
//! a restart of the compositor without sill.

use std::io::Read;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use super::randr::{Head, parse_heads};
use super::scale::DisplayScale;
use crate::error::ServiceError;

/// How long [`run`] waits for `cosmic-randr` to answer before killing it. On a compositor
/// without `zwlr_output_manager_v1` (KWin) `cosmic-randr` does not fail promptly as this
/// module's doc once assumed: it hangs, its stdout pipe never closes, and
/// `std::process::Command::output` (with no bound of its own) waits forever. That parks the
/// `displays` service's blocking-pool task forever too, which in turn parks the daemon's async
/// runtime at shutdown: `sill quit` answers over IPC but the process never exits (FINDINGS G51).
const COSMIC_RANDR_TIMEOUT: Duration = Duration::from_secs(2);

/// The compositor's outputs, and the one change sill makes to them.
pub trait ScaleSink {
    /// Every output the compositor reports now.
    fn heads(&mut self) -> Result<Vec<Head>, ServiceError>;
    /// Run `head` at `scale`, keeping its current mode.
    fn set_scale(&mut self, head: &Head, scale: DisplayScale) -> Result<(), ServiceError>;
}

/// `cosmic-randr`, on `$PATH`, against the compositor `$WAYLAND_DISPLAY` names (the one sill
/// itself is connected to). Under a compositor without `zwlr_output_manager_v1` (KWin) it
/// hangs rather than failing (FINDINGS G51); [`COSMIC_RANDR_TIMEOUT`] bounds the wait, and past
/// it the service changes nothing.
#[derive(Debug, Clone, Default)]
pub struct CosmicRandr;

impl ScaleSink for CosmicRandr {
    fn heads(&mut self) -> Result<Vec<Head>, ServiceError> {
        run(&["list", "--kdl"]).map(|text| parse_heads(&text))
    }

    fn set_scale(&mut self, head: &Head, scale: DisplayScale) -> Result<(), ServiceError> {
        let args = mode_args(head, scale).ok_or(ServiceError::Unsupported(
            "an output mode to keep while scaling",
        ))?;
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        run(&args).map(|_| ())
    }
}

/// `mode <name> <w> <h> --refresh <hz> --scale <s>`: the current mode, re-applied at `scale`.
/// `None` for a head with no current mode (disabled).
pub fn mode_args(head: &Head, scale: DisplayScale) -> Option<Vec<String>> {
    let mode = head.current?;
    let mut args = vec![
        "mode".to_owned(),
        head.name.0.clone(),
        mode.pixels.width.to_string(),
        mode.pixels.height.to_string(),
    ];
    if mode.refresh_mhz > 0 {
        args.push("--refresh".to_owned());
        args.push(format!(
            "{}.{:03}",
            mode.refresh_mhz / 1000,
            mode.refresh_mhz % 1000
        ));
    }
    args.push("--scale".to_owned());
    args.push(scale.decimal());
    Some(args)
}

fn run(args: &[&str]) -> Result<String, ServiceError> {
    run_tool("cosmic-randr", args)
}

/// `program args`'s stdout, waited for at most [`COSMIC_RANDR_TIMEOUT`]; a launch failure, a
/// timeout or a non-zero exit is an error naming the command. Also what reads
/// `kscreen-doctor --json` ([`super::kscreen::KscreenDoctor`]).
pub(super) fn run_tool(program: &str, args: &[&str]) -> Result<String, ServiceError> {
    let mut command = Command::new(program);
    command.args(args);
    let output = wait_bounded(command, COSMIC_RANDR_TIMEOUT)
        .map_err(|error| ServiceError::Launch(format!("{program} {}: {error}", args.join(" "))))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(ServiceError::Launch(format!(
            "{program} {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

/// `command`'s output, polled rather than awaited outright: past `timeout` the child is killed
/// and reaped instead of left to block the caller forever, unlike `Command::output`. The output
/// collected is whatever the child wrote before that (empty on a timeout).
fn wait_bounded(mut command: Command, timeout: Duration) -> std::io::Result<Output> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Output {
                status,
                stdout: drain(&mut child, ChildPipe::Stdout),
                stderr: drain(&mut child, ChildPipe::Stderr),
            });
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::other(format!("no answer in {timeout:?}")));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

enum ChildPipe {
    Stdout,
    Stderr,
}

/// Every byte a finished child wrote to `pipe`; empty if it wrote none or the pipe was already
/// taken.
fn drain(child: &mut Child, pipe: ChildPipe) -> Vec<u8> {
    let mut buf = Vec::new();
    let read = match pipe {
        ChildPipe::Stdout => child.stdout.take().map(|mut out| out.read_to_end(&mut buf)),
        ChildPipe::Stderr => child.stderr.take().map(|mut err| err.read_to_end(&mut buf)),
    };
    let _ = read;
    buf
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::{Duration, Instant};

    use super::{mode_args, wait_bounded};
    use crate::cosmic_wl::OutputName;
    use crate::displays::edid::{Mode, Pixels};
    use crate::displays::randr::Head;
    use crate::displays::scale::DisplayScale;

    /// A child that answers well inside the bound is waited for and read normally, the way a
    /// real `cosmic-randr` does on cosmic-comp.
    #[test]
    fn a_child_that_answers_is_read_normally() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf hi; printf bye >&2"]);
        let output =
            wait_bounded(command, Duration::from_secs(5)).unwrap_or_else(|e| panic!("{e}"));
        assert!(output.status.success());
        assert_eq!(output.stdout, b"hi");
        assert_eq!(output.stderr, b"bye");
    }

    /// A child that never answers (FINDINGS G51: `cosmic-randr` under KWin) is killed at the
    /// bound rather than waited for forever, and `wait_bounded` returns well within it.
    #[test]
    fn a_child_that_never_answers_is_killed_at_the_bound() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 30"]);
        let bound = Duration::from_millis(200);
        let started = Instant::now();
        let error = wait_bounded(command, bound).expect_err("a sleeping child never answers");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "not killed promptly"
        );
        assert!(error.to_string().contains("no answer"), "{error}");
    }

    #[test]
    fn a_scale_change_keeps_the_current_mode() {
        let head = |refresh_mhz| Head {
            name: OutputName("HDMI-A-2".to_owned()),
            current: Some(Mode {
                pixels: Pixels {
                    width: 3840,
                    height: 2160,
                },
                refresh_mhz,
            }),
            ..Head::default()
        };
        let cases: Vec<(Head, DisplayScale, Option<&str>)> = vec![
            (
                head(59997),
                DisplayScale(150),
                Some("mode HDMI-A-2 3840 2160 --refresh 59.997 --scale 1.5"),
            ),
            (
                head(60000),
                DisplayScale(200),
                Some("mode HDMI-A-2 3840 2160 --refresh 60.000 --scale 2"),
            ),
            (
                head(0),
                DisplayScale(125),
                Some("mode HDMI-A-2 3840 2160 --scale 1.25"),
            ),
            (Head::default(), DisplayScale(150), None),
        ];
        for (head, scale, want) in cases {
            assert_eq!(
                mode_args(&head, scale).map(|a| a.join(" ")).as_deref(),
                want,
                "{want:?}"
            );
        }
    }
}
