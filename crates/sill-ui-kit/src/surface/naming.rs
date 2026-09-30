//! Layer namespaces: `sill-<surface>-<output>`, the name `sill debug` addresses a surface by.

use shell_host::OutputInfo;

/// `sill-<surface>-<output name>`, or `sill-<surface>-<output id>` for an unnamed output.
pub fn namespace(surface: &str, output: &OutputInfo) -> String {
    match &output.name {
        Some(name) => format!("sill-{surface}-{name}"),
        None => format!("sill-{surface}-{}", output.id.0),
    }
}

#[cfg(test)]
mod tests {
    use super::namespace;
    use crate::harness::fake::output_named as output;

    #[test]
    fn namespaces_name_the_output() {
        assert_eq!(namespace("bar", &output(Some("DP-1"))), "sill-bar-DP-1");
        assert_eq!(namespace("dock", &output(None)), "sill-dock-7");
    }
}
