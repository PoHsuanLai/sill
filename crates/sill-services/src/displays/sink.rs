//! Where an output's scale is read and set: the [`ScaleSink`] seam, and its one real side,
//! [`CosmicRandr`], which runs the `cosmic-randr` tool COSMIC ships. cosmic-comp persists what
//! it applies in `$XDG_STATE_HOME/cosmic-comp/outputs.ron` itself, so a scale set here survives
//! a restart of the compositor without sill.

use std::process::Command;

use super::randr::{Head, parse_heads};
use super::scale::DisplayScale;
use crate::error::ServiceError;

/// The compositor's outputs, and the one change sill makes to them.
pub trait ScaleSink {
    /// Every output the compositor reports now.
    fn heads(&mut self) -> Result<Vec<Head>, ServiceError>;
    /// Run `head` at `scale`, keeping its current mode.
    fn set_scale(&mut self, head: &Head, scale: DisplayScale) -> Result<(), ServiceError>;
}

/// `cosmic-randr`, on `$PATH`, against the compositor `$WAYLAND_DISPLAY` names (the one sill
/// itself is connected to). Under a compositor without `zwlr_output_manager_v1` (KWin) it
/// fails, and the service changes nothing.
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
    let output = Command::new("cosmic-randr")
        .args(args)
        .output()
        .map_err(|e| ServiceError::Launch(format!("cosmic-randr: {e}")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(ServiceError::Launch(format!(
            "cosmic-randr {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::mode_args;
    use crate::cosmic_wl::OutputName;
    use crate::displays::edid::{Mode, Pixels};
    use crate::displays::randr::Head;
    use crate::displays::scale::DisplayScale;

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
