//! A display's identity across reconnects, ports and machines: its EDID make, model and
//! serial, reduced to a key that names the model in clear and hashes the serial away.

use serde::{Deserialize, Serialize};

use super::edid::{Edid, Serial};

/// `BNQ-8074-1f2e3d4c5b6a7980`: manufacturer, product code, and a 64-bit FNV-1a hash of
/// manufacturer, product and serial together. The key `displays.json` and
/// `display.scale_overrides` use; stable across releases (the hash is fixed here, not
/// `std`'s randomised one).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IdentityKey(pub String);

impl IdentityKey {
    /// The key for what an EDID says.
    pub fn of_edid(edid: &Edid) -> IdentityKey {
        let serial = match &edid.serial {
            Serial::Text(text) => text.clone(),
            Serial::Number(n) => format!("#{n}"),
        };
        IdentityKey::of_parts(&edid.manufacturer.0, edid.product.0, &serial)
    }

    /// The key for an output with no EDID of its own (a nested compositor's window, a virtual
    /// output): the make and model the compositor reports, with no product code or serial.
    pub fn of_head(make: &str, model: &str) -> IdentityKey {
        IdentityKey::of_parts(make, 0, model)
    }

    fn of_parts(make: &str, product: u16, serial: &str) -> IdentityKey {
        let text = format!("{make}\u{1f}{product}\u{1f}{serial}");
        let make: String = make
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(8)
            .collect();
        IdentityKey(format!(
            "{make}-{product:04x}-{:016x}",
            fnv1a(text.as_bytes())
        ))
    }
}

/// 64-bit FNV-1a: tiny, stable across builds, and plenty for telling displays apart.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::{IdentityKey, fnv1a};
    use crate::displays::edid::parse;
    use crate::displays::edid::tests::MA270U;

    #[test]
    fn fnv1a_matches_its_reference_vectors() {
        const CASES: &[(&[u8], u64)] = &[
            (b"", 0xcbf2_9ce4_8422_2325),
            (b"a", 0xaf63_dc4c_8601_ec8c),
            (b"foobar", 0x8594_4171_f739_67e8),
        ];
        for &(input, want) in CASES {
            assert_eq!(fnv1a(input), want, "{input:?}");
        }
    }

    #[test]
    fn keys_name_the_model_and_hide_the_serial() {
        let edid = parse(MA270U).unwrap_or_else(|e| panic!("{e}"));
        let key = IdentityKey::of_edid(&edid);
        assert!(key.0.starts_with("BNQ-8074-"), "{key:?}");
        assert!(!key.0.contains("SERIAL"), "{key:?}");
        assert_eq!(key.0.len(), "BNQ-8074-".len() + 16, "{key:?}");

        let mut other = edid.clone();
        other.serial = crate::displays::edid::Serial::Text("SERIAL0000002".to_owned());
        assert_ne!(IdentityKey::of_edid(&other), key, "two panels of one model");
        assert_eq!(IdentityKey::of_edid(&edid), key, "stable");

        let head = IdentityKey::of_head("Example", "WINIT-0");
        assert!(head.0.starts_with("Example-0000-"), "{head:?}");
    }
}
