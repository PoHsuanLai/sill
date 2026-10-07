//! The scale route against a live cosmic-comp: `cosmic-randr` lists the heads, sets a scale,
//! and the compositor reports it back; then the original is put back. Refuses to run unless
//! every head is a nested compositor's window (`WINIT-*`), so it can never rescale a real
//! display.
//!
//! `WAYLAND_DISPLAY=wayland-N cargo test -p sill-services --test it scale_live -- --ignored
//! --nocapture` (with `dev/nested.sh` running on wayland-N).

use sill_model::displays::DisplayScale;
use sill_services::displays::{CosmicRandr, ScaleSink};

#[test]
#[ignore = "changes the nested compositor's output scale"]
fn a_scale_set_through_cosmic_randr_is_what_the_compositor_reports() {
    let mut sink = CosmicRandr;
    let heads = sink
        .heads()
        .unwrap_or_else(|e| panic!("cosmic-randr list: {e}"));
    println!("heads: {heads:#?}");
    assert!(
        !heads.is_empty(),
        "no heads: is WAYLAND_DISPLAY the nested compositor?"
    );
    assert!(
        heads.iter().all(|h| h.name.0.starts_with("WINIT-")),
        "refusing: a head is not a nested window ({heads:?})"
    );
    let head = heads[0].clone();
    let original = head.scale;
    let wanted = if original == DisplayScale(125) {
        DisplayScale(150)
    } else {
        DisplayScale(125)
    };

    sink.set_scale(&head, wanted)
        .unwrap_or_else(|e| panic!("set {}: {e}", wanted.decimal()));
    let after = sink.heads().unwrap_or_else(|e| panic!("{e}"));
    println!("after setting {}: {after:#?}", wanted.decimal());
    let now = after.iter().find(|h| h.name == head.name).map(|h| h.scale);

    sink.set_scale(&head, original)
        .unwrap_or_else(|e| panic!("restore {}: {e}", original.decimal()));
    let restored = sink.heads().unwrap_or_else(|e| panic!("{e}"));
    let back = restored
        .iter()
        .find(|h| h.name == head.name)
        .map(|h| h.scale);

    assert_eq!(now, Some(wanted), "the compositor did not take the scale");
    assert_eq!(back, Some(original), "the original scale was not restored");
}
