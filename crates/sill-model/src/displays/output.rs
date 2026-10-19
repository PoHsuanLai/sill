//! One output as the displays service describes it: the compositor's head joined with what the
//! display's own EDID says. Pure: [`describe`].

use serde::{Deserialize, Serialize};

use super::drm::{Connector, Placement, placement};
use super::edid;
use super::edid::{Edid, Millimetres, Mode, Primaries};
use super::gamut::{Gamut, classify};
use super::identity::IdentityKey;
use super::ppi::{Ppi, density};
use super::randr::Head;
use super::scale::DisplayScale;
use crate::ids::OutputName;

/// One output and everything sill derives from it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayOutput {
    pub name: OutputName,
    pub identity: IdentityKey,
    /// The EDID product name, else the compositor's model.
    pub product: String,
    pub placement: Placement,
    pub size: Option<Millimetres>,
    /// The panel's native mode (EDID), else the compositor's current one.
    pub native: Option<Mode>,
    pub density: Option<Ppi>,
    /// `None` when there is no EDID to read them from.
    pub primaries: Option<Primaries>,
    pub gamut: Gamut,
    /// What the compositor runs it at now.
    pub scale: DisplayScale,
}

/// `head` described, using `connector`'s EDID when it has a readable one. The EDID wins for
/// size, native mode and identity: the compositor's copy of the same facts may be rounded, and
/// a nested compositor's window has no EDID at all.
pub fn describe(head: &Head, connector: Option<&Connector>) -> DisplayOutput {
    let edid: Option<Edid> = connector.and_then(|c| edid::parse(&c.edid).ok());
    let size = edid.as_ref().and_then(|e| e.size).or(head.physical);
    let native = edid.as_ref().and_then(|e| e.native).or(head.current);
    let pixels = native.map(|m| m.pixels).unwrap_or_default();
    DisplayOutput {
        name: head.name.clone(),
        identity: edid.as_ref().map_or_else(
            || IdentityKey::of_head(&head.make, &head.model),
            IdentityKey::of_edid,
        ),
        product: edid
            .as_ref()
            .map(|e| e.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| head.model.clone()),
        placement: placement(&head.name),
        size,
        native,
        density: density(pixels, size),
        primaries: edid.as_ref().map(|e| e.primaries),
        gamut: edid
            .as_ref()
            .map_or(Gamut::Srgb, |e| classify(&e.primaries)),
        scale: head.scale,
    }
}

/// The densest output's density: what the font-rendering default follows.
pub fn max_density(outputs: &[DisplayOutput]) -> Option<Ppi> {
    outputs.iter().filter_map(|o| o.density).max()
}

#[cfg(test)]
mod tests {
    use super::{describe, max_density};
    use crate::displays::drm::{Connector, Placement};
    use crate::displays::edid::tests::MA270U;
    use crate::displays::edid::{Millimetres, Mode, Pixels};
    use crate::displays::gamut::Gamut;
    use crate::displays::ppi::Ppi;
    use crate::displays::randr::Head;
    use crate::displays::scale::DisplayScale;
    use crate::ids::OutputName;

    fn head(name: &str, width: u32, height: u32) -> Head {
        Head {
            name: OutputName(name.to_owned()),
            make: "Example".to_owned(),
            model: name.to_owned(),
            current: Some(Mode {
                pixels: Pixels { width, height },
                refresh_mhz: 60000,
            }),
            scale: DisplayScale(100),
            ..Head::default()
        }
    }

    #[test]
    fn a_head_with_an_edid_is_described_by_the_edid() {
        let connector = Connector {
            name: OutputName("HDMI-A-2".to_owned()),
            edid: MA270U.to_vec(),
        };
        let out = describe(&head("HDMI-A-2", 2560, 1440), Some(&connector));
        assert_eq!(out.product, "BenQ MA270U");
        assert!(
            out.identity.0.starts_with("BNQ-8074-"),
            "{:?}",
            out.identity
        );
        assert_eq!(out.native.map(|m| m.pixels.width), Some(3840));
        assert_eq!(out.density, Some(Ppi(163)));
        assert_eq!(out.gamut, Gamut::DisplayP3);
        assert_eq!(out.scale, DisplayScale(100));
        assert_eq!(out.placement, Placement::External);
    }

    #[test]
    fn a_head_without_one_falls_back_to_the_compositor() {
        let nested = describe(&head("WINIT-0", 2176, 1360), None);
        assert_eq!(nested.product, "WINIT-0");
        assert_eq!(nested.density, None);
        assert_eq!(nested.gamut, Gamut::Srgb);
        assert_eq!(nested.primaries, None);

        let garbage = Connector {
            name: OutputName("eDP-1".to_owned()),
            edid: vec![1, 2, 3],
        };
        let mut laptop = head("eDP-1", 2880, 1800);
        laptop.physical = Some(Millimetres {
            width: 302,
            height: 189,
        });
        let laptop = describe(&laptop, Some(&garbage));
        assert_eq!(laptop.placement, Placement::Internal);
        assert_eq!(max_density(&[nested, laptop]), Some(Ppi(242)));
    }
}
