//! Launcher v2 on the real panel in a `ds_native::Harness`: the runner's sections
//! with Top Hit and "Show More", the preview pane's keys (Space while browsing, ⌘Y, → at the end,
//! Escape one level at a time), a row chord run from the results, Tab to one provider, and the
//! rows `sill debug launcher-rows` reports. The preview rows are the fakes' (`launcher_support/m9.rs`).

mod launcher_harness;

use std::time::Duration;

use ds::ShortcutKey;
use launcher_harness::launcher_support::m9;
use launcher_harness::{Open, Press, Seen, await_off_thread, is_open, open, settle};
use sill_launcher::{OpenTarget, PreviewKind};
use sill_model::launch::LaunchCommand;
use sill_model::results::SearchCommand;
use sill_ui_kit::links::ShownRows;

fn rows(open: &Open) -> ShownRows {
    open.link.rows().expect("the open panel publishes its rows")
}

/// Press `key` on the harness's keyboard.
fn press(open: &mut Open, key: Press) {
    open.press(key);
    open.harness.advance(Duration::from_millis(40));
    open.harness.advance(Duration::from_millis(20));
}

fn headings(open: &Open) -> Vec<String> {
    rows(open).sections.into_iter().map(|s| s.title).collect()
}

#[test]
fn m9_lists_top_hit_then_documents_capped_with_show_more() {
    let mut open = open("m9");
    assert_eq!(
        headings(&open),
        ["Top Hit", "Documents", "Clipboard", "Web"]
    );
    let shown = rows(&open);
    assert_eq!(shown.sections[0].rows, ["m9-report.pdf"]);
    assert_eq!(shown.sections[1].hidden, 2, "seven documents, five shown");
    assert_eq!(
        shown.sections[1].rows.last().map(String::as_str),
        Some("Show More")
    );
    assert_eq!(
        shown.sections[2].rows,
        ["<clipboard>"],
        "no clipboard text leaves the panel"
    );
    // Down to the Show More line (Top Hit, five documents: line 6) and Enter: all seven show.
    for _ in 0..6 {
        press(&mut open, Press::Down);
    }
    assert_eq!(rows(&open).selected, 6);
    press(&mut open, Press::Enter);
    let shown = rows(&open);
    assert!(is_open(&open.link), "Show More keeps the launcher open");
    assert_eq!(shown.sections[1].hidden, 0);
    assert_eq!(
        shown.sections[1].rows.len(),
        8,
        "seven documents and Show Less"
    );
    assert_eq!(
        shown.sections[1].rows.last().map(String::as_str),
        Some("Show Less")
    );
}

#[test]
fn space_while_browsing_shows_the_pdf_and_then_the_image() {
    let mut open = open("m9");
    // The harness's field opens with its caret at the start; a person's ends at the query.
    open.harness.key(ShortcutKey::End);
    press(&mut open, Press::Space);
    assert_eq!(rows(&open).preview, None, "typing: Space is a space");
    assert_eq!(rows(&open).query, "m9 ");
    press(&mut open, Press::Up);
    press(&mut open, Press::Space);
    assert_eq!(
        rows(&open).preview,
        Some(PreviewKind::Pdf),
        "browsing: Quick Look"
    );
    // The page rasters off the harness's thread; the pending look holds the pane until it lands.
    let landed = await_off_thread(&mut open.harness, |h| h.count(".ds-pdf-thumb") == 1);
    assert_eq!(landed, Seen::Once, "quire's PDF thumbnail");
    press(&mut open, Press::Down);
    assert_eq!(
        rows(&open).preview,
        Some(PreviewKind::Image),
        "the pane follows the row"
    );
    press(&mut open, Press::Space);
    assert_eq!(rows(&open).preview, None, "Space again hides it");
}

#[test]
fn command_y_while_typing_then_right_enter_the_pane_and_escape_steps_out() {
    let mut open = open("m9");
    open.harness
        .chord(&[ShortcutKey::Ctrl], ShortcutKey::Char('y'));
    open.harness.advance(Duration::from_millis(40));
    assert_eq!(
        rows(&open).preview,
        Some(PreviewKind::Pdf),
        "⌘Y while typing"
    );
    let html = open.harness.html();
    assert!(html.contains("ds-preview"), "quire's pane");
    assert!(html.contains("ds-palette-aside"), "as the palette's aside");
    assert!(
        !html.contains("data-focused=\"true\""),
        "the results have the keyboard"
    );
    // `sill debug launcher-key right` is → at the field's end.
    press(&mut open, Press::Right);
    let html = open.harness.html();
    assert!(html.contains("Reveal in Files"), "the pane's controls");
    assert!(
        open.harness.count(".ds-preview .ds-chord") > 0 && html.contains("⌘"),
        "with their chords, as plain text after the label (F1040)"
    );
    assert_eq!(open.harness.count(".ds-preview .ds-kbd"), 0, "no key caps");
    assert!(
        html.contains("data-focused=\"true\""),
        "→ at the end, with the pane shown, gave its first control the keyboard"
    );
    press(&mut open, Press::Escape);
    assert!(
        !open.harness.html().contains("data-focused=\"true\""),
        "back to the results"
    );
    assert_eq!(
        rows(&open).preview,
        Some(PreviewKind::Pdf),
        "the pane stays"
    );
    press(&mut open, Press::Escape);
    assert_eq!(rows(&open).preview, None, "Escape hides the pane");
    assert!(is_open(&open.link));
    press(&mut open, Press::Escape);
    assert!(!is_open(&open.link), "then Escape closes the launcher");
}

#[test]
fn a_real_right_enters_the_pane_only_from_the_fields_end() {
    let mut open = open("m9");
    open.harness
        .chord(&[ShortcutKey::Ctrl], ShortcutKey::Char('y'));
    open.harness.advance(Duration::from_millis(40));
    // The field opens with its caret at the end (as Spotlight); Home puts it at the
    // start, where → moves the caret rather than entering the pane.
    open.harness.key(ShortcutKey::Home);
    open.harness.key(ShortcutKey::Right);
    open.harness.advance(Duration::from_millis(40));
    assert!(
        !open.harness.html().contains("data-focused=\"true\""),
        "the caret moved"
    );
    open.harness.key(ShortcutKey::End);
    open.harness.key(ShortcutKey::Right);
    open.harness.advance(Duration::from_millis(40));
    assert!(
        open.harness.html().contains("data-focused=\"true\""),
        "→ at the end entered the pane"
    );
    assert_eq!(rows(&open).query, "m9", "the field took no key");
}

#[test]
fn a_real_space_after_a_real_down_is_quick_look() {
    let mut open = open("m9");
    open.harness.key(ShortcutKey::Down);
    open.harness.key(ShortcutKey::Up);
    open.harness.advance(Duration::from_millis(40));
    open.harness.key(ShortcutKey::Space);
    open.harness.advance(Duration::from_millis(40));
    let shown = rows(&open);
    assert_eq!(
        shown.preview,
        Some(PreviewKind::Pdf),
        "browsing: Quick Look"
    );
    assert_eq!(shown.selected, 0, "the selection stays on the row");
    assert_eq!(
        shown.query, "m9",
        "the claim took the Space: the field typed none"
    );
}

#[test]
fn a_real_escape_in_the_field_hides_the_pane_before_it_closes() {
    let mut open = open("m9");
    open.harness
        .chord(&[ShortcutKey::Ctrl], ShortcutKey::Char('y'));
    open.harness.advance(Duration::from_millis(40));
    assert_eq!(rows(&open).preview, Some(PreviewKind::Pdf));
    open.harness.key(ShortcutKey::Escape);
    open.harness.advance(Duration::from_millis(40));
    assert!(is_open(&open.link), "the first Escape was the pane's");
    assert_eq!(rows(&open).preview, None);
    open.harness.key(ShortcutKey::Escape);
    let link = open.link.clone();
    settle(&mut open.harness, |_| !is_open(&link));
    assert!(!is_open(&open.link));
}

#[test]
fn command_r_reveals_the_selected_file() {
    let mut open = open("m9");
    open.harness
        .chord(&[ShortcutKey::Ctrl], ShortcutKey::Char('r'));
    let link = open.link.clone();
    settle(&mut open.harness, |_| !is_open(&link));
    match open.feeds.bar.launch.try_recv() {
        Ok(LaunchCommand::Open(request)) => assert_eq!(
            request.target,
            OpenTarget::Reveal(m9::folder().join("m9-report.pdf"))
        ),
        other => panic!("no reveal: {other:?}"),
    }
}

#[test]
fn shift_tab_with_nothing_typed_reaches_emoji_alone() {
    let mut open = open("");
    press(&mut open, Press::ShiftTab);
    press(&mut open, Press::ShiftTab);
    settle(&mut open.harness, |h| h.count(".ds-emoji-cell") == 3);
    let shown = rows(&open);
    assert_eq!(headings(&open), ["Emoji"]);
    assert_eq!(shown.sections[0].rows, ["🎉", "👍", "🦊"]);
    let searches = open
        .feeds
        .searches
        .lock()
        .map(|l| l.clone())
        .unwrap_or_default();
    assert!(
        searches.iter().any(|c| matches!(
            c,
            SearchCommand::Filtered {
                provider: sill_launcher::ProviderKind::Emoji,
                ..
            }
        )),
        "the filter asks the emoji provider alone: {searches:?}"
    );
    assert_eq!(
        open.harness.count(".ds-emoji-cell"),
        3,
        "quire's emoji grid"
    );
}

/// On the emoji grid a real → moves the cell (design/13 §13.3.9, 2D arrow keys) rather than
/// showing the preview; ← moves it back; Space after the move is Quick Look (the move was a
/// browse), and ⌘Y and ⌘→ still reach the pane there.
#[test]
fn real_left_and_right_move_the_emoji_grid_and_command_y_previews() {
    let mut open = open("");
    press(&mut open, Press::ShiftTab);
    press(&mut open, Press::ShiftTab);
    settle(&mut open.harness, |h| h.count(".ds-emoji-cell") == 3);
    assert_eq!(headings(&open), ["Emoji"]);
    assert_eq!(rows(&open).selected, 0);
    open.harness.key(ShortcutKey::Right);
    open.harness.advance(Duration::from_millis(40));
    let shown = rows(&open);
    assert_eq!(shown.selected, 1, "→ moved to the next cell");
    assert_eq!(shown.preview, None, "→ on a cell is not the preview's");
    open.harness.key(ShortcutKey::Right);
    open.harness.key(ShortcutKey::Left);
    open.harness.advance(Duration::from_millis(40));
    assert_eq!(rows(&open).selected, 1, "→ then ← came back");
    open.harness.key(ShortcutKey::Space);
    open.harness.advance(Duration::from_millis(40));
    assert_eq!(
        rows(&open).preview,
        Some(PreviewKind::Emoji),
        "browsing the grid: Space is Quick Look"
    );
    open.harness.key(ShortcutKey::Right);
    open.harness.advance(Duration::from_millis(40));
    let shown = rows(&open);
    assert_eq!(shown.selected, 2, "→ still moves with the pane shown");
    assert_eq!(shown.preview, Some(PreviewKind::Emoji), "the pane follows");
    open.harness
        .chord(&[ShortcutKey::Ctrl], ShortcutKey::Char('y'));
    open.harness.advance(Duration::from_millis(40));
    assert_eq!(rows(&open).preview, None, "⌘Y hides it");
    open.harness
        .chord(&[ShortcutKey::Ctrl], ShortcutKey::Char('y'));
    open.harness.advance(Duration::from_millis(40));
    assert_eq!(rows(&open).preview, Some(PreviewKind::Emoji), "⌘Y shows it");
    open.harness.chord(&[ShortcutKey::Ctrl], ShortcutKey::Right);
    open.harness.advance(Duration::from_millis(40));
    assert!(
        open.harness.html().contains("data-focused=\"true\""),
        "⌘→ gave the pane's first control the keyboard"
    );
    assert_eq!(rows(&open).selected, 2, "⌘→ did not move the cell");
}
