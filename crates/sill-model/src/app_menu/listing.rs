//! The focused app's menus as text for an agent: one line per row, the id `shell.menu.activate`
//! takes first. The words are the app's own, so the caller labels the whole text untrusted.

use crate::tray::{Check, TrayMenuEntry, TrayMenuKind};
use crate::units::Availability;

use super::route::ItemKey;
use super::titles::{shown, titles};
use super::{OwnGroup, ServedMenu};

/// The most characters a listing has; a longer one ends at a line with a mark that it was cut.
pub const LISTING_MAX: usize = 12_000;

const CUT: &str = "(more rows left out)";

/// The menus of the app called `name`: its own menu (the app's first menu, then the shell's own
/// `own` rows), then each menu the app sent.
pub fn listing(name: &str, own: &[OwnGroup], served: Option<&ServedMenu>) -> String {
    let sent = served.map(titles).unwrap_or_default();
    let mut lines = vec![name.to_owned()];
    if let Some((_, rows)) = &sent.app {
        rows_lines(rows, 1, &mut lines);
    }
    for row in own.iter().flatten() {
        lines.push(format!("  [{}] {}", ItemKey::Own(row.item).text(), row.label));
    }
    for menu in &sent.menus {
        lines.push(menu.title.clone());
        rows_lines(&menu.rows, 1, &mut lines);
    }
    cut(lines)
}

fn rows_lines(rows: &[TrayMenuEntry], depth: usize, lines: &mut Vec<String>) {
    for row in rows.iter().filter(|row| shown(row)) {
        let TrayMenuKind::Item {
            label,
            enabled,
            check,
            children,
            shortcut,
            command,
            ..
        } = &row.kind
        else {
            continue;
        };
        let mut line = format!("{}[{}] {label}", "  ".repeat(depth), row.id);
        if *enabled == Availability::Disabled {
            line.push_str(" (disabled)");
        }
        match check {
            Check::None => {}
            Check::Unchecked => line.push_str(" (unchecked)"),
            Check::Checked => line.push_str(" (checked)"),
            Check::Mixed => line.push_str(" (mixed)"),
        }
        if let Some(keys) = shortcut {
            line.push_str(&format!(" keys={}", keys.0.join("+")));
        }
        if let Some(command) = command {
            line.push_str(&format!(" command={}", command.0));
        }
        lines.push(line);
        rows_lines(children, depth + 1, lines);
    }
}

/// `lines` joined, ending at a whole line within [`LISTING_MAX`].
fn cut(lines: Vec<String>) -> String {
    let mut text = String::new();
    for line in lines {
        if text.chars().count() + line.chars().count() + 1 + CUT.len() > LISTING_MAX {
            text.push_str(CUT);
            return text;
        }
        text.push_str(&line);
        text.push('\n');
    }
    text
}
