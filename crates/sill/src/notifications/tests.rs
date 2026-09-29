use shell_host::{OutputId, OutputInfo, Rect, Scale, SurfaceId};
use sill_ipc::{ReportHolder, ReportMode};
use sill_model::notifications::{NameHolder, NameOwner, Notifications, ServerMode, ServerStatus};
use sill_surfaces::surfaces::banners::BannerOutput;

use super::{banner_output, namespaces, report_of};
use crate::outputs::{OutputSurfaces, Outputs};

fn output(id: u32, name: &str) -> OutputInfo {
    OutputInfo {
        id: OutputId(id),
        name: Some(name.to_owned()),
        description: None,
        logical: Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        },
        scale: Scale::ONE,
        refresh_mhz: 60_000,
    }
}

fn row(banners: Option<u64>) -> OutputSurfaces {
    OutputSurfaces {
        banners: banners.map(SurfaceId),
        ..OutputSurfaces::default()
    }
}

#[test]
fn banners_show_on_the_first_output_that_has_a_column() {
    let cases = [
        (vec![], BannerOutput::Nowhere),
        (vec![(output(1, "DP-1"), row(None))], BannerOutput::Nowhere),
        (
            vec![
                (output(1, "DP-1"), row(None)),
                (output(2, "DP-2"), row(Some(9))),
            ],
            BannerOutput::On(OutputId(2)),
        ),
        (
            vec![
                (output(1, "DP-1"), row(Some(3))),
                (output(2, "DP-2"), row(Some(9))),
            ],
            BannerOutput::On(OutputId(1)),
        ),
    ];
    for (rows, want) in cases {
        let outputs = Outputs { outputs: rows };
        assert_eq!(banner_output(&outputs), want, "{outputs:?}");
    }
}

#[test]
fn the_status_reports_the_holder_and_the_counts() {
    let live = Notifications {
        server: ServerStatus {
            mode: ServerMode::Own,
            holder: NameHolder::Other(NameOwner {
                unique: ":1.9".to_owned(),
                pid: Some(42),
                command: Some("plasmashell".to_owned()),
            }),
        },
        banners: Vec::new(),
    };
    let report = report_of(&live, &Default::default());
    assert_eq!(report.mode, ReportMode::Own);
    assert_eq!(
        report.holder,
        ReportHolder::Other {
            unique: ":1.9".to_owned(),
            pid: Some(42),
            command: Some("plasmashell".to_owned()),
        }
    );
    assert_eq!((report.banners, report.kept), (0, 0));
    let passive = report_of(&Notifications::default(), &Default::default());
    assert_eq!(passive.mode, ReportMode::Passive);
    assert_eq!(passive.holder, ReportHolder::NoBus);
}

#[test]
fn the_surfaces_are_named_for_debug() {
    assert_eq!(
        namespaces(&output(1, "DP-1")),
        (
            "sill-banners-DP-1".to_owned(),
            "sill-notification-center-DP-1".to_owned()
        )
    );
}
