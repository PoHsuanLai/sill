//! The body as quire draws it: the service's parsed markup (`sill_model::notifications::Body`, `<b>`, `<i>`,
//! `<u>`, `<a href>`) as quire's [`Rich`] runs. A run has one tone in quire, so a run that is
//! several at once takes the strongest: a link, then bold, then italic, then underline. The line
//! clamp and its fade are quire's (`NotificationCard`), so nothing is clipped here.

use ds::components::content::rich_text::{Rich, RichRun};
use ds::components::content::text_runs::RunTone;
use ds::prelude::*;
use sill_model::notifications::{Body, BodyRun, RunLink, RunStyle, Slant, Underline, Weight};

/// `body` as quire's runs; `None` for an empty body (the card then draws no body line).
pub fn rich_body(body: &Body) -> Option<Rich> {
    let runs: Vec<RichRun> = body
        .runs
        .iter()
        .filter(|run| !run.text.is_empty())
        .map(rich_run)
        .collect();
    (!runs.is_empty()).then_some(Rich(runs))
}

fn rich_run(run: &BodyRun) -> RichRun {
    match &run.style.link {
        RunLink::To(href) => RichRun::link(run.text.clone(), href.0.to_string()),
        RunLink::Plain => RichRun::Run(TextRun::new(run.text.clone(), tone(&run.style))),
    }
}

/// The one tone a run is drawn in.
fn tone(style: &RunStyle) -> RunTone {
    match (style.weight, style.slant, style.underline) {
        (Weight::Bold, _, _) => RunTone::Strong,
        (Weight::Regular, Slant::Italic, _) => RunTone::Italic,
        (Weight::Regular, Slant::Upright, Underline::Single) => RunTone::Underline,
        (Weight::Regular, Slant::Upright, Underline::None) => RunTone::Plain,
    }
}

#[cfg(test)]
mod tests {
    use super::rich_body;
    use ds::components::content::rich_text::{Rich, RichRun};
    use ds::components::content::text_runs::RunTone;
    use ds::prelude::*;
    use sill_model::notifications::{Body, parse_body};

    fn run(text: &str, tone: RunTone) -> RichRun {
        RichRun::Run(TextRun::new(text, tone))
    }

    #[test]
    fn the_services_markup_becomes_quires_runs() {
        let cases: Vec<(&str, Option<Rich>)> = vec![
            ("", None),
            ("plain", Some(Rich(vec![run("plain", RunTone::Plain)]))),
            (
                "<b>bold</b> <i>slant</i> <u>under</u>",
                Some(Rich(vec![
                    run("bold", RunTone::Strong),
                    run(" ", RunTone::Plain),
                    run("slant", RunTone::Italic),
                    run(" ", RunTone::Plain),
                    run("under", RunTone::Underline),
                ])),
            ),
            (
                "<b><i>both</i></b>",
                Some(Rich(vec![run("both", RunTone::Strong)])),
            ),
            (
                "<i><u>both</u></i>",
                Some(Rich(vec![run("both", RunTone::Italic)])),
            ),
            (
                "see <a href=\"https://x.org\"><b>the log</b></a>",
                Some(Rich(vec![
                    run("see ", RunTone::Plain),
                    RichRun::link("the log", "https://x.org"),
                ])),
            ),
        ];
        for (raw, want) in cases {
            assert_eq!(rich_body(&parse_body(raw)), want, "{raw:?}");
        }
        assert_eq!(rich_body(&Body::plain("a <b>")), Some(Rich::from("a <b>")));
    }
}
