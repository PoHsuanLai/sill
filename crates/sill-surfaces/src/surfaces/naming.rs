//! Layer namespaces: `sill-<surface>-<output>`, the name `sill debug` addresses a surface by.

use shell_host::OutputInfo;

/// `sill-<surface>-<output name>`, or `sill-<surface>-<output id>` for an unnamed output.
pub(crate) fn namespace(surface: &str, output: &OutputInfo) -> String {
    match &output.name {
        Some(name) => format!("sill-{surface}-{name}"),
        None => format!("sill-{surface}-{}", output.id.0),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::namespace;
    use shell_host::{OutputId, OutputInfo, Rect, Scale};

    /// A 1920 x 1080 output named `name`, for every surface's tests.
    pub(crate) fn output(name: Option<&str>) -> OutputInfo {
        OutputInfo {
            id: OutputId(7),
            name: name.map(str::to_owned),
            description: None,
            logical: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            scale: Scale::ONE,
            refresh_mhz: 60_000,
        }
    }

    #[test]
    fn namespaces_name_the_output() {
        assert_eq!(namespace("bar", &output(Some("DP-1"))), "sill-bar-DP-1");
        assert_eq!(namespace("dock", &output(None)), "sill-dock-7");
    }
}
