use shell_host::{LayerExtent, LayerSize, OutputId, OutputInfo, Rect, Scale, SurfaceId};
use sill_ui_kit::links::BannerOutput;

use super::banner_output;
use crate::outputs::{OutputSurfaces, Outputs, Spawned};

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
        spawned: banners
            .map(|id| Spawned {
                name: "banners",
                slot: "()".to_owned(),
                id: SurfaceId(id),
                namespace: "sill-banners-DP-1".to_owned(),
                size: LayerSize {
                    width: LayerExtent::Px(440),
                    height: LayerExtent::Fill,
                },
            })
            .into_iter()
            .collect(),
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
