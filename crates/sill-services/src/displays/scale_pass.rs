//! One pass over the compositor's outputs: describe each, decide its scale
//! ([`seen::decide_scale`]), set it through the [`ScaleSink`], and return the new record. The
//! only effects are the sink's; the caller loads and saves the record.
//!
//! When the sink has no answer (KWin: no `zwlr_output_manager_v1`, or no `cosmic-randr`), the
//! outputs come from a read-only [`HeadReader`] (`kscreen-doctor --json`) instead, and the pass
//! only describes them: no scale is decided, set or remembered, so KDE's own scaling is never
//! changed and a later COSMIC session still sees each display for the first time.

use super::drm::Connector;
use super::kscreen::HeadReader;
use super::output::{DisplayOutput, describe};
use super::policy::DisplayPolicy;
use super::randr::{Enablement, Head};
use super::seen::{ScaleDecision, ScaleFacts, SeenDisplay, SeenDisplays, decide_scale};
use super::sink::ScaleSink;
use crate::error::ServiceError;

/// What one pass did.
#[derive(Debug)]
pub struct Pass {
    /// The enabled outputs, described, with any scale this pass set.
    pub outputs: Vec<DisplayOutput>,
    pub seen: SeenDisplays,
    /// Every failure, for the caller to log.
    pub failures: Vec<ServiceError>,
    pub source: HeadSource,
}

/// Where a pass's outputs came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadSource {
    /// The sink (`cosmic-randr`): the scale policy applies.
    Sink,
    /// The read-only fallback (`kscreen-doctor`): described only, no scale policy.
    Reader,
    /// Neither answered: nothing described, nothing changed.
    Neither,
}

/// Run one pass: the sink's heads with the scale policy, else `fallback`'s heads described
/// read-only, else nothing.
pub fn scale_pass<S: ScaleSink, R: HeadReader>(
    sink: &mut S,
    fallback: &mut R,
    connectors: &[Connector],
    seen: SeenDisplays,
    policy: &DisplayPolicy,
) -> Pass {
    match sink.heads() {
        Ok(heads) => apply(sink, &heads, connectors, seen, policy),
        Err(sink_error) => match fallback.heads() {
            Ok(heads) => Pass {
                outputs: enabled(&heads)
                    .map(|head| {
                        let connector = connectors.iter().find(|c| c.name == head.name);
                        describe(
                            head,
                            connector,
                            policy.density_target,
                            policy.scale_rounding,
                        )
                    })
                    .collect(),
                seen,
                failures: Vec::new(),
                source: HeadSource::Reader,
            },
            Err(reader_error) => Pass {
                outputs: Vec::new(),
                seen,
                failures: vec![sink_error, reader_error],
                source: HeadSource::Neither,
            },
        },
    }
}

fn enabled(heads: &[Head]) -> impl Iterator<Item = &Head> {
    heads.iter().filter(|h| h.enablement == Enablement::Enabled)
}

fn apply<S: ScaleSink>(
    sink: &mut S,
    heads: &[Head],
    connectors: &[Connector],
    seen: SeenDisplays,
    policy: &DisplayPolicy,
) -> Pass {
    enabled(heads).fold(
        Pass {
            outputs: Vec::new(),
            seen,
            failures: Vec::new(),
            source: HeadSource::Sink,
        },
        |pass, head| one(sink, connectors, policy, pass, head),
    )
}

fn one<S: ScaleSink>(
    sink: &mut S,
    connectors: &[Connector],
    policy: &DisplayPolicy,
    mut pass: Pass,
    head: &Head,
) -> Pass {
    let connector = connectors.iter().find(|c| c.name == head.name);
    let mut output = describe(
        head,
        connector,
        policy.density_target,
        policy.scale_rounding,
    );
    let record = pass.seen.displays.get(&output.identity);
    let decision = decide_scale(ScaleFacts {
        mode: policy.scale,
        seen: record.map(|r| r.applied),
        requested: policy.override_for(&output.identity),
        computed: output.default_scale,
        current: output.scale,
    });
    let applied = match decision {
        ScaleDecision::Leave => None,
        ScaleDecision::Record(applied) => Some(applied),
        ScaleDecision::Apply(scale) => match sink.set_scale(head, scale) {
            Ok(()) => {
                output.scale = scale;
                Some(Some(scale))
            }
            Err(error) => {
                pass.failures.push(error);
                None
            }
        },
    };
    if let Some(applied) = applied {
        pass.seen.displays.insert(
            output.identity.clone(),
            SeenDisplay {
                name: output.product.clone(),
                applied,
            },
        );
    }
    pass.outputs.push(output);
    pass
}

#[cfg(test)]
mod tests;
