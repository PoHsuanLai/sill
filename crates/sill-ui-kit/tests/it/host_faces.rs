//! Every sill process registers quire's faces with shell-host: a document under the host that
//! names Inter must find Inter, not the system face.

use std::path::Path;

use shell_host::registered_families;
use sill_services::env::Dirs;

#[test]
fn the_host_config_registers_quires_faces() {
    let families = registered_families(&sill_ui_kit::root::host_config(&Dirs::under(Path::new(
        "/scratch",
    ))));
    for family in ["Inter", "Inter Display", "Space Mono"] {
        assert!(
            families.iter().any(|known| known == family),
            "{family} is registered (got {families:?})"
        );
    }
}
