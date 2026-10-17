//! Which apps `openers_of` lists (a host-only app opens nothing here) and which `offered_apps`
//! offers to a helper.

use docket_core::{Manifest, validate};

use super::{offered_apps, openers_of};

fn manifest(visibility: &str) -> docket_core::ValidManifest {
    let text = format!(
        r#"
vocab = 1
app = "org.quire.Mail"
visibility = "{visibility}"

[[entities]]
kind = "mail.thread"
label = "Thread"
plural = "Threads"
icon = "mail"
class = "mail"
index = "indexed"
titles = {{ kind = "third_party", v = {{ kind = "mail" }} }}
props = []

[[actions]]
name = "mail.thread.open"
label = "Open"
on = {{ kind = "one", v = "mail.thread" }}
effect = "read"
classes = []
undo = "not_undoable"
reach = "hidden"
latency = "quick"
result = {{ kind = "nothing" }}
keys = {{ kind = "none" }}
lasting = "no"
dry_run = "none"
params = []
"#
    );
    let parsed: Manifest = toml::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
    validate(parsed).unwrap_or_else(|e| panic!("{e:?}"))
}

#[test]
fn an_app_for_everyone_is_listed() {
    assert_eq!(openers_of(&[manifest("everyone")]).len(), 1);
}

#[test]
fn a_host_only_app_is_not_listed() {
    assert!(openers_of(&[manifest("host_only")]).is_empty());
}

#[test]
fn a_helper_is_offered_the_apps_for_everyone_and_not_the_host_only_ones() {
    assert_eq!(offered_apps(&[manifest("everyone")]).len(), 1);
    assert!(offered_apps(&[manifest("host_only")]).is_empty());
}
