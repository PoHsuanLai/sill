//! The seams a surface root shares with the host: the rect measurer, the focus write and the
//! caret read every root is given, quire's faces, and the pre-paint pass. A popup's root
//! component calls `ds_native::measure::provide()` and `ds_native::focus::provide()` instead.

/// The rect measurer every root the daemon spawns is given (`Root::context(MEASURE)`): quire's
/// own, `ds_native::measure::MEASURE`. A popup's root
/// component calls `ds_native::measure::provide()` instead.
pub use ds_native::measure::MEASURE;

/// The pre-paint pass the daemon installs on its host (`HostConfig::prepaint`): quire's
/// device-pixel layout snap, a no-op at whole scales.
pub fn prepaint() -> shell_host::Prepaint {
    shell_host::Prepaint::hook(ds_native::snap_to_device)
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

/// quire's caret read (`HostCaret`), for a root to carry beside [`FOCUS`] so a palette's
/// key claim sees where the caret is. The launcher's panel provides it itself
/// (`ds_native::focus::provide()`), so it does not wait on the daemon's roots.
pub use ds_native::focus::CARET;
/// quire's focus write every root the daemon spawns is given beside [`MEASURE`]
/// (`Root::context(FOCUS)`), so a field's or a menu's focus waits out a busy document instead
/// of printing a caught panic. A popup's root
/// component calls `ds_native::focus::provide()` instead.
pub use ds_native::focus::FOCUS;
