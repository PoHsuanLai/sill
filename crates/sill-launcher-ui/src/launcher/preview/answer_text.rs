//! A text answer in the preview pane: the question as the heading, then every line of the
//! answer in order, its emphasis set in quire's text runs (`ds-run`), never as markup.

use dioxus::prelude::*;
use ds::components::content::text_runs::{RunTone, TextLine, TextRun, text};
use sill_model::companion::{AnswerFacts, Emphasis, Span, body_lines, spans};

/// The answer's lines as paragraphs under the question.
pub(super) fn answer_text(answer: &AnswerFacts) -> Element {
    let lines: Vec<TextLine> = body_lines(answer)
        .iter()
        .map(|line| runs_of(line))
        .collect();
    rsx! {
        div { class: "sill-answer-text",
            h2 { class: "sill-answer-title", "{answer.prompt}" }
            for (at , line) in lines.iter().enumerate() {
                p { key: "{at}", class: "sill-answer-line", {text(line)} }
            }
        }
    }
}

fn runs_of(line: &str) -> TextLine {
    TextLine::Runs(spans(line).into_iter().map(run_of).collect())
}

fn run_of(span: Span) -> TextRun {
    let tone = match span.emphasis {
        Emphasis::Plain => RunTone::Plain,
        Emphasis::Strong => RunTone::Strong,
        Emphasis::Italic => RunTone::Italic,
        Emphasis::Code => RunTone::Code,
    };
    TextRun::new(span.text, tone)
}
