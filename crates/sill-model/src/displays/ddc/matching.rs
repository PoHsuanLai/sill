//! Which i2c bus is which monitor. Some drivers link a connector to its bus in sysfs
//! (`card0-DP-1/ddc`); others (NVIDIA's) do not, so the reliable join is the EDID: the bytes a
//! bus returns from address 0x50 are the bytes the kernel shows for exactly one connector.
//! Pure.

use crate::ids::{DdcId, OutputName};

/// `i2c-5` is bus 5.
pub fn bus_number(entry: &str) -> Option<DdcId> {
    entry.strip_prefix("i2c-")?.parse().ok().map(DdcId)
}

/// Whether a PCI class (`0x030000`, as sysfs prints it) is a display controller (class 0x03:
/// VGA, XGA, 3D), whose i2c buses are the ones that reach monitors.
pub fn is_display_class(class: &str) -> bool {
    class
        .trim()
        .strip_prefix("0x")
        .is_some_and(|hex| hex.len() == 6 && hex.starts_with("03"))
}

/// The connector whose EDID's base block equals `bus_edid`'s, if exactly one does.
pub fn connector_for<'a>(
    bus_edid: &[u8],
    connectors: &'a [(OutputName, Vec<u8>)],
) -> Option<&'a OutputName> {
    let block = bus_edid.get(..128)?;
    let mut matches = connectors
        .iter()
        .filter(|(_, edid)| edid.get(..128) == Some(block))
        .map(|(name, _)| name);
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

#[cfg(test)]
mod tests {
    use super::{bus_number, connector_for, is_display_class};
    use crate::displays::edid::tests::MA270U;
    use crate::ids::DdcId;
    use crate::ids::OutputName;

    #[test]
    fn names_and_classes_parse_at_their_boundaries() {
        assert_eq!(bus_number("i2c-5"), Some(DdcId(5)));
        assert_eq!(bus_number("i2c-12"), Some(DdcId(12)));
        assert_eq!(bus_number("i2c-"), None);
        assert_eq!(bus_number("i2c-dev"), None);
        assert_eq!(bus_number("spi-1"), None);
        const CLASSES: &[(&str, bool)] = &[
            ("0x030000\n", true),
            ("0x030200", true),
            ("0x060400", false),
            ("0x0300", false),
            ("030000", false),
            ("0x0c0330", false),
        ];
        for &(class, want) in CLASSES {
            assert_eq!(is_display_class(class), want, "{class:?}");
        }
    }

    #[test]
    fn a_bus_joins_the_connector_with_its_edid() {
        let other = {
            let mut b = MA270U.to_vec();
            b[12] ^= 1;
            b
        };
        let name = |n: &str| OutputName(n.to_owned());
        let connectors = vec![
            (name("DP-1"), other.clone()),
            (name("HDMI-A-2"), [MA270U.as_slice(), &[0u8; 128]].concat()),
        ];
        assert_eq!(connector_for(MA270U, &connectors), Some(&name("HDMI-A-2")));
        assert_eq!(connector_for(&other, &connectors), Some(&name("DP-1")));
        assert_eq!(connector_for(&[0u8; 128], &connectors), None);
        assert_eq!(connector_for(&MA270U[..64], &connectors), None);
        let twins = vec![
            (name("DP-1"), MA270U.to_vec()),
            (name("DP-2"), MA270U.to_vec()),
        ];
        assert_eq!(connector_for(MA270U, &twins), None, "ambiguous");
    }
}
