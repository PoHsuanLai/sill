//! Built-in panels' backlights: read from `/sys/class/backlight`, set through logind's
//! `Session.SetBrightness`, which lets the seat's own session write the value without any
//! group membership or udev rule (systemd-logind >= 243).

use std::path::Path;

use logind_zbus::session::SessionProxy;

use super::brightness::{Backlight, BacklightKind, BacklightName};
use crate::env::Bus;
use crate::error::ServiceError;

/// Every backlight under `root` (`/sys/class/backlight`) that reads whole, in name order.
pub fn read_all(root: &Path) -> Vec<Backlight> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let number = |dir: &Path, file: &str| {
        std::fs::read_to_string(dir.join(file))
            .ok()
            .and_then(|t| t.trim().parse::<u32>().ok())
    };
    let mut found: Vec<Backlight> = entries
        .flatten()
        .filter_map(|entry| {
            let dir = entry.path();
            Some(Backlight {
                name: BacklightName(entry.file_name().to_string_lossy().into_owned()),
                kind: BacklightKind::parse(&std::fs::read_to_string(dir.join("type")).ok()?)?,
                value: number(&dir, "brightness")?,
                max: number(&dir, "max_brightness")?,
            })
        })
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// Set backlight `name` to the raw `value` through logind, as the caller's own session.
pub async fn set(system: &Bus, name: &BacklightName, value: u32) -> Result<(), ServiceError> {
    let connection = system
        .connect()
        .await?
        .ok_or(ServiceError::NoBus(system.kind()))?;
    let session = SessionProxy::builder(&connection)
        .path("/org/freedesktop/login1/session/auto")?
        .build()
        .await?;
    session
        .set_brightness("backlight", &name.0, value)
        .await
        .map_err(ServiceError::from)
}

#[cfg(test)]
mod tests {
    use super::read_all;
    use crate::displays::brightness::{Backlight, BacklightKind, BacklightName};

    #[test]
    fn backlights_read_from_a_sysfs_like_tree() {
        let root = std::env::temp_dir().join(format!("sill-backlight-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |dir: &str, files: &[(&str, &str)]| {
            let dir = root.join(dir);
            std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
            for (name, text) in files {
                std::fs::write(dir.join(name), text).unwrap_or_else(|e| panic!("{e}"));
            }
        };
        write(
            "intel_backlight",
            &[
                ("type", "raw\n"),
                ("brightness", "9600\n"),
                ("max_brightness", "19200\n"),
            ],
        );
        write(
            "acpi_video0",
            &[
                ("type", "firmware\n"),
                ("brightness", "5\n"),
                ("max_brightness", "10\n"),
            ],
        );
        write(
            "broken",
            &[
                ("type", "raw\n"),
                ("brightness", "x\n"),
                ("max_brightness", "10\n"),
            ],
        );
        let want = vec![
            Backlight {
                name: BacklightName("acpi_video0".to_owned()),
                kind: BacklightKind::Firmware,
                value: 5,
                max: 10,
            },
            Backlight {
                name: BacklightName("intel_backlight".to_owned()),
                kind: BacklightKind::Raw,
                value: 9600,
                max: 19200,
            },
        ];
        assert_eq!(read_all(&root), want);
        assert_eq!(read_all(&root.join("missing")), vec![]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
