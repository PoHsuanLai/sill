use super::{HeadSource, scale_pass};
use crate::cosmic_wl::OutputName;
use crate::displays::drm::Connector;
use crate::displays::edid::tests::MA270U;
use crate::displays::edid::{Mode, Pixels};
use crate::displays::kscreen::HeadReader;
use crate::displays::policy::{DisplayPolicy, ScaleMode, ScaleOverride};
use crate::displays::randr::{Enablement, Head};
use crate::displays::scale::DisplayScale;
use crate::displays::seen::SeenDisplays;
use crate::displays::sink::ScaleSink;
use crate::error::ServiceError;

/// A compositor made of a list: `set_scale` changes the head's scale and is logged; `broken`
/// makes every set fail the way `cosmic-randr` under KWin does; `listing` `Silent` makes the
/// list itself fail, as `cosmic-randr list --kdl` does on KWin ("no answer in 2s").
#[derive(Debug, Default)]
struct FakeSink {
    heads: Vec<Head>,
    sets: Vec<(String, DisplayScale)>,
    broken: Broken,
    listing: Listing,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Listing {
    #[default]
    Answers,
    Silent,
}

/// A fallback that never answers (no `kscreen-doctor`).
#[derive(Debug)]
struct NoReader;

impl HeadReader for NoReader {
    fn heads(&mut self) -> Result<Vec<Head>, ServiceError> {
        Err(ServiceError::Launch("kscreen-doctor: not found".to_owned()))
    }
}

/// A fallback that reads these heads, and counts how often it was asked.
#[derive(Debug, Default)]
struct FakeReader {
    heads: Vec<Head>,
    reads: usize,
}

impl HeadReader for FakeReader {
    fn heads(&mut self) -> Result<Vec<Head>, ServiceError> {
        self.reads += 1;
        Ok(self.heads.clone())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Broken {
    #[default]
    Works,
    Fails,
}

impl ScaleSink for FakeSink {
    fn heads(&mut self) -> Result<Vec<Head>, ServiceError> {
        match self.listing {
            Listing::Answers => Ok(self.heads.clone()),
            Listing::Silent => Err(ServiceError::Launch(
                "cosmic-randr list --kdl: no answer in 2s".to_owned(),
            )),
        }
    }

    fn set_scale(&mut self, head: &Head, scale: DisplayScale) -> Result<(), ServiceError> {
        if self.broken == Broken::Fails {
            return Err(ServiceError::Unsupported("zwlr_output_manager_v1"));
        }
        self.sets.push((head.name.0.clone(), scale));
        for h in self.heads.iter_mut().filter(|h| h.name == head.name) {
            h.scale = scale;
        }
        Ok(())
    }
}

fn hdmi(scale: u16) -> Head {
    Head {
        name: OutputName("HDMI-A-2".to_owned()),
        enablement: Enablement::Enabled,
        make: "BenQ".to_owned(),
        model: "MA270U".to_owned(),
        physical: None,
        current: Some(Mode {
            pixels: Pixels {
                width: 3840,
                height: 2160,
            },
            refresh_mhz: 60000,
        }),
        scale: DisplayScale(scale),
    }
}

fn connectors() -> Vec<Connector> {
    vec![Connector {
        name: OutputName("HDMI-A-2".to_owned()),
        edid: MA270U.to_vec(),
    }]
}

#[test]
fn first_sight_sets_the_default_and_never_again() {
    let mut sink = FakeSink {
        heads: vec![hdmi(100)],
        ..FakeSink::default()
    };
    let policy = DisplayPolicy::default();
    let first = scale_pass(
        &mut sink,
        &mut NoReader,
        &connectors(),
        SeenDisplays::default(),
        &policy,
    );
    // PreferWhole (the default): the MA270U's raw 1.499 rounds to the whole-number branch, and
    // 3840 / 2 = 1920 stays well above MIN_LOGICAL_WIDTH.
    assert_eq!(sink.sets, vec![("HDMI-A-2".to_owned(), DisplayScale(200))]);
    assert_eq!(first.outputs[0].scale, DisplayScale(200));
    let record = first.seen.displays.get(&first.outputs[0].identity).cloned();
    assert_eq!(record.map(|r| r.applied), Some(Some(DisplayScale(200))));

    // The person picks 1.75 in COSMIC Settings; the next pass leaves it.
    sink.heads = vec![hdmi(175)];
    let second = scale_pass(
        &mut sink,
        &mut NoReader,
        &connectors(),
        first.seen.clone(),
        &policy,
    );
    assert_eq!(sink.sets.len(), 1, "no second set");
    assert_eq!(second.outputs[0].scale, DisplayScale(175));
    assert_eq!(second.seen, first.seen);
}

#[test]
fn manual_records_without_setting_and_overrides_still_apply() {
    let mut sink = FakeSink {
        heads: vec![hdmi(100)],
        ..FakeSink::default()
    };
    let manual = DisplayPolicy {
        scale: ScaleMode::Manual,
        ..DisplayPolicy::default()
    };
    let pass = scale_pass(
        &mut sink,
        &mut NoReader,
        &connectors(),
        SeenDisplays::default(),
        &manual,
    );
    assert_eq!(sink.sets, vec![]);
    let identity = pass.outputs[0].identity.clone();
    assert_eq!(
        pass.seen.displays.get(&identity).map(|r| r.applied),
        Some(None)
    );

    let row = format!("{} = 2", identity.0);
    let with_override = DisplayPolicy {
        scale_overrides: ScaleOverride::parse(&row).into_iter().collect(),
        ..manual
    };
    let pass = scale_pass(
        &mut sink,
        &mut NoReader,
        &connectors(),
        pass.seen,
        &with_override,
    );
    assert_eq!(sink.sets, vec![("HDMI-A-2".to_owned(), DisplayScale(200))]);
    let again = scale_pass(
        &mut sink,
        &mut NoReader,
        &connectors(),
        pass.seen,
        &with_override,
    );
    assert_eq!(sink.sets.len(), 1, "an applied override is not re-applied");
    assert!(again.failures.is_empty());
}

#[test]
fn a_failed_set_records_nothing_so_the_next_pass_retries() {
    let mut sink = FakeSink {
        heads: vec![hdmi(100)],
        broken: Broken::Fails,
        ..FakeSink::default()
    };
    let pass = scale_pass(
        &mut sink,
        &mut NoReader,
        &connectors(),
        SeenDisplays::default(),
        &DisplayPolicy::default(),
    );
    assert_eq!(pass.failures.len(), 1);
    assert_eq!(pass.seen, SeenDisplays::default());
    assert_eq!(pass.outputs[0].scale, DisplayScale(100));
}

#[test]
fn a_nested_window_without_edid_stays_at_one() {
    let mut sink = FakeSink {
        heads: vec![Head {
            name: OutputName("WINIT-0".to_owned()),
            make: "COSMIC".to_owned(),
            model: "WINIT-0".to_owned(),
            current: Some(Mode {
                pixels: Pixels {
                    width: 2176,
                    height: 1360,
                },
                refresh_mhz: 60000,
            }),
            scale: DisplayScale(100),
            ..Head::default()
        }],
        ..FakeSink::default()
    };
    let pass = scale_pass(
        &mut sink,
        &mut NoReader,
        &connectors(),
        SeenDisplays::default(),
        &DisplayPolicy::default(),
    );
    assert_eq!(sink.sets, vec![]);
    assert_eq!(pass.outputs[0].default_scale, DisplayScale(100));
    assert_eq!(pass.seen.displays.len(), 1, "recorded as seen");
}

/// On KWin the outputs come from the reader, described but never rescaled: under every scale
/// mode and even with an override for the display, no set reaches the sink and nothing is
/// remembered, so KDE's scaling stays the user's and a later COSMIC session still gets its
/// first-sight scale.
#[test]
fn a_silent_sink_falls_back_to_the_reader_read_only() {
    let identity = {
        let mut sink = FakeSink {
            heads: vec![hdmi(100)],
            ..FakeSink::default()
        };
        let manual = DisplayPolicy {
            scale: ScaleMode::Manual,
            ..DisplayPolicy::default()
        };
        let pass = scale_pass(
            &mut sink,
            &mut NoReader,
            &connectors(),
            SeenDisplays::default(),
            &manual,
        );
        pass.outputs[0].identity.clone()
    };
    let with_override = DisplayPolicy {
        scale_overrides: vec![ScaleOverride {
            display: identity,
            scale: DisplayScale(150),
        }],
        ..DisplayPolicy::default()
    };
    let policies: Vec<(&str, DisplayPolicy)> = vec![
        ("auto", DisplayPolicy::default()),
        (
            "manual",
            DisplayPolicy {
                scale: ScaleMode::Manual,
                ..DisplayPolicy::default()
            },
        ),
        ("an override", with_override),
    ];
    for (what, policy) in policies {
        let mut sink = FakeSink {
            listing: Listing::Silent,
            ..FakeSink::default()
        };
        let mut reader = FakeReader {
            heads: vec![
                hdmi(200),
                Head {
                    name: OutputName("DP-4".to_owned()),
                    enablement: Enablement::Disabled,
                    ..Head::default()
                },
            ],
            ..FakeReader::default()
        };
        let pass = scale_pass(
            &mut sink,
            &mut reader,
            &connectors(),
            SeenDisplays::default(),
            &policy,
        );
        assert_eq!(pass.source, HeadSource::Reader, "{what}");
        assert_eq!(sink.sets, vec![], "{what}: no scale set on KWin");
        assert_eq!(
            pass.seen,
            SeenDisplays::default(),
            "{what}: nothing remembered"
        );
        assert!(pass.failures.is_empty(), "{what}: {:?}", pass.failures);
        let names: Vec<&str> = pass.outputs.iter().map(|o| o.name.0.as_str()).collect();
        assert_eq!(names, vec!["HDMI-A-2"], "{what}: the enabled output only");
        assert_eq!(pass.outputs[0].scale, DisplayScale(200), "{what}");
        assert_eq!(
            pass.outputs[0].product, "BenQ MA270U",
            "{what}: from the EDID"
        );
    }
}

#[test]
fn the_reader_is_asked_only_when_the_sink_is_silent_and_both_failing_changes_nothing() {
    let mut reader = FakeReader {
        heads: vec![hdmi(200)],
        ..FakeReader::default()
    };
    let mut sink = FakeSink {
        heads: vec![hdmi(100)],
        ..FakeSink::default()
    };
    let pass = scale_pass(
        &mut sink,
        &mut reader,
        &connectors(),
        SeenDisplays::default(),
        &DisplayPolicy::default(),
    );
    assert_eq!(pass.source, HeadSource::Sink);
    assert_eq!(reader.reads, 0);

    let mut silent = FakeSink {
        listing: Listing::Silent,
        ..FakeSink::default()
    };
    let pass = scale_pass(
        &mut silent,
        &mut NoReader,
        &connectors(),
        SeenDisplays::default(),
        &DisplayPolicy::default(),
    );
    assert_eq!(pass.source, HeadSource::Neither);
    assert_eq!(pass.outputs, vec![]);
    assert_eq!(pass.failures.len(), 2, "both reasons logged");
    assert_eq!(silent.sets, vec![]);
}
