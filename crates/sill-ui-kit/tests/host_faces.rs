//! Every sill process registers quire's faces with shell-host: a document under the host that
//! names Inter must find Inter, not the system face.

use shell_host::registered_families;

#[test]
fn the_host_config_registers_quires_faces() {
    let families = registered_families(&sill_ui_kit::root::host_config());
    for family in ["Inter", "Inter Display", "Space Mono"] {
        assert!(
            families.iter().any(|known| known == family),
            "{family} is registered (got {families:?})"
        );
    }
}
