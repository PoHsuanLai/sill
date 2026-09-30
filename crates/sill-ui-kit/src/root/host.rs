//! What a sill process hands shell-host: quire's faces and the pre-paint pass.

/// The pre-paint pass the daemon installs on its host (`HostConfig::prepaint`): quire's
/// device-pixel layout snap, a no-op at whole scales.
pub fn prepaint() -> shell_host::Prepaint {
    shell_host::Prepaint::hook(ds_blitz::snap_to_device)
}

/// quire's bundled faces (Inter, Inter Display, Space Mono), for `HostConfig::fonts`. Without
/// them every document under shell-host falls back to the system face, since none of them is
/// installed on the system.
pub fn faces() -> shell_host::FontFaces {
    shell_host::FontFaces(ds::FACES.iter().map(|face| face.bytes).collect())
}

/// The host configuration every sill process starts from: quire's faces and the pre-paint
/// pass. The daemon adds its settings on top; the demos use it as it is.
pub fn host_config() -> shell_host::HostConfig {
    shell_host::HostConfig {
        fonts: faces(),
        prepaint: prepaint(),
        ..shell_host::HostConfig::default()
    }
}
