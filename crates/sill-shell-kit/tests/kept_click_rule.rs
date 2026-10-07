//! The kept-click rule, held over the shell parts' sources (FINDINGS "Native focus"): a click
//! handler that stops a click's propagation keeps it from the `Ds` root's click-focus fallback,
//! so it must hand the click over itself with `kept_click`, or the keyboard ends up nowhere on
//! Blitz (mailo, against v0.1.10). A handler named by function (`onclick: fence`) is read from
//! that function's body in the same file.

use std::path::{Path, PathBuf};

/// Every `.rs` file under `dir`, recursively.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|error| panic!("{dir:?}: {error}"));
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .flat_map(|path| match path.is_dir() {
            true => sources(&path),
            false if path.extension().is_some_and(|ext| ext == "rs") => vec![path],
            false => Vec::new(),
        })
        .collect()
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The lines from `start` to the line that closes it at `start`'s indentation, or `start`
/// alone when it closes itself (`onclick: fence,`).
fn block(lines: &[&str], start: usize) -> String {
    let first = lines[start];
    let opens = first.trim_end().ends_with('{') || first.trim_end().ends_with('(');
    if !opens {
        return first.to_owned();
    }
    let depth = indent(first);
    let end = lines[start + 1..]
        .iter()
        .position(|line| indent(line) == depth && line.trim_start().starts_with(['}', ')']))
        .map_or(lines.len(), |at| start + 1 + at);
    lines[start..=end.min(lines.len() - 1)].join("\n")
}

/// The body of `fn name(` in `lines`, if the file defines it.
fn function(lines: &[&str], name: &str) -> Option<String> {
    let head = format!("fn {name}(");
    lines
        .iter()
        .position(|line| line.contains(&head))
        .map(|at| block(lines, at))
}

/// Each `onclick` handler in a file, as source text, a named handler resolved to its function.
fn click_handlers(text: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with("onclick:"))
        .map(|(at, line)| {
            let named = line
                .trim()
                .trim_start_matches("onclick:")
                .trim()
                .trim_end_matches(',');
            let is_name =
                !named.is_empty() && named.chars().all(|c| c.is_alphanumeric() || c == '_');
            let body = match is_name {
                true => function(&lines, named).unwrap_or_else(|| block(&lines, at)),
                false => block(&lines, at),
            };
            (at + 1, body)
        })
        .collect()
}

#[test]
fn every_click_a_component_stops_is_handed_to_the_click_focus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let broken: Vec<String> = sources(&root.join("src"))
        .iter()
        .flat_map(|path| {
            let text = std::fs::read_to_string(path).unwrap_or_default();
            click_handlers(&text)
                .into_iter()
                .filter(|(_, body)| {
                    body.contains("stop_propagation") && !body.contains("kept_click")
                })
                .map(|(line, _)| format!("{}:{line}", path.display()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        broken.is_empty(),
        "these click handlers stop the click without `kept_click`: {broken:#?}"
    );
}

/// The scan itself finds what it is meant to: a stopped handler without `kept_click`, inline or
/// named, and passes one that has it.
#[test]
fn the_scan_sees_a_stopped_click() {
    let inline = "        onclick: move |event| event.stop_propagation(),\n";
    assert!(click_handlers(inline)[0].1.contains("stop_propagation"));
    let named = "    span {\n        onclick: fence,\n    }\nfn fence(event: MouseEvent) {\n    event.stop_propagation();\n}\n";
    let found = click_handlers(named);
    assert!(found[0].1.contains("stop_propagation") && !found[0].1.contains("kept_click"));
    let kept = "        onclick: move |event| {\n            event.stop_propagation();\n            kept_click(&event);\n        },\n";
    assert!(click_handlers(kept)[0].1.contains("kept_click"));
}
