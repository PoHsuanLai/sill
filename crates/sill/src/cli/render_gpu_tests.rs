//! `decide` over table rows, and `scan` over made-up sysfs trees.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use super::{DeviceName, Gpu, Verdict, decide, is_pci_address, scan};

fn gpu(card: &str, device: &str, connected: usize) -> Gpu {
    Gpu {
        card: card.to_owned(),
        device: DeviceName(device.to_owned()),
        connected,
    }
}

#[test]
fn the_verdict_follows_where_the_outputs_are() {
    let amd = gpu("card1", "pci-0000:10:00.0", 1);
    let nvidia = gpu("card2", "pci-0000:01:00.0", 0);
    let cases = [
        ("no GPU", vec![], Verdict::Alone),
        ("one GPU", vec![amd.clone()], Verdict::Alone),
        (
            "the monitor on the second",
            vec![nvidia.clone(), amd.clone()],
            Verdict::Pin(amd.clone()),
        ),
        (
            "nothing connected",
            vec![nvidia.clone(), gpu("card3", "pci-0000:03:00.0", 0)],
            Verdict::NoDisplay,
        ),
        (
            "outputs on two",
            vec![amd.clone(), gpu("card2", "pci-0000:01:00.0", 2)],
            Verdict::Split(vec!["card1".to_owned(), "card2".to_owned()]),
        ),
        (
            "outputs on three",
            vec![amd.clone(), gpu("card2", "x", 1), gpu("card3", "y", 1)],
            Verdict::Split(vec!["card1".into(), "card2".into(), "card3".into()]),
        ),
    ];
    for (name, gpus, want) in cases {
        assert_eq!(decide(&gpus), want, "{name}");
    }
}

#[test]
fn only_a_pin_prints_a_device() {
    let amd = gpu("card1", "pci-0000:10:00.0", 1);
    assert_eq!(
        Verdict::Pin(amd).device(),
        Some(&DeviceName("pci-0000:10:00.0".to_owned()))
    );
    for verdict in [Verdict::Alone, Verdict::NoDisplay, Verdict::Split(vec![])] {
        assert_eq!(verdict.device(), None);
        assert!(verdict.reason().contains("left to cosmic-comp"));
    }
}

#[test]
fn pci_addresses_are_recognised() {
    for (name, want) in [
        ("0000:10:00.0", true),
        ("0000:0a:1f.7", true),
        ("platform-fe000000.gpu", false),
        ("0000:10:00", false),
        ("0000:10:00.0.1", false),
        ("000:10:00.0", false),
        ("", false),
    ] {
        assert_eq!(is_pci_address(name), want, "{name}");
    }
}

/// A scratch sysfs root, removed on drop.
struct Tree(PathBuf);

impl Tree {
    fn new(name: &str) -> Tree {
        let root = std::env::temp_dir().join(format!("sill-gpu-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("class/drm")).unwrap_or_else(|e| panic!("{e}"));
        Tree(root)
    }

    /// `class/drm/<card>` with its `device` symlink to `devices/<device>` that holds `drm/<render>`.
    fn card(&self, card: &str, device: &str, render: &str) -> &Tree {
        let dev = self.0.join("devices").join(device);
        fs::create_dir_all(dev.join("drm").join(render)).unwrap_or_else(|e| panic!("{e}"));
        fs::create_dir_all(dev.join("drm").join(card)).unwrap_or_else(|e| panic!("{e}"));
        let class = self.0.join("class/drm").join(card);
        fs::create_dir_all(&class).unwrap_or_else(|e| panic!("{e}"));
        symlink(&dev, class.join("device")).unwrap_or_else(|e| panic!("{e}"));
        self
    }

    /// `class/drm/<connector>/status`.
    fn connector(&self, connector: &str, status: &str) -> &Tree {
        let dir = self.0.join("class/drm").join(connector);
        fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        fs::write(dir.join("status"), format!("{status}\n")).unwrap_or_else(|e| panic!("{e}"));
        self
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_hybrid_desktop_pins_the_gpu_with_the_monitor() {
    let tree = Tree::new("hybrid");
    tree.card("card1", "pci0000:00/0000:10:00.0", "renderD128")
        .card("card2", "pci0000:00/0000:01:00.0", "renderD129")
        .connector("card1-HDMI-A-1", "connected")
        .connector("card1-Writeback-1", "unknown")
        .connector("card2-DP-1", "disconnected")
        .connector("card2-DP-2", "disconnected");
    let gpus = scan(tree.path());
    assert_eq!(
        gpus,
        vec![
            gpu("card1", "pci-0000:10:00.0", 1),
            gpu("card2", "pci-0000:01:00.0", 0)
        ]
    );
    assert_eq!(decide(&gpus), Verdict::Pin(gpus[0].clone()));
}

#[test]
fn card1_does_not_claim_card10s_connectors_and_a_card_without_a_render_node_is_skipped() {
    let tree = Tree::new("prefix");
    tree.card("card1", "pci0000:00/0000:10:00.0", "renderD128")
        .card("card10", "pci0000:00/0000:20:00.0", "renderD129")
        .connector("card10-DP-1", "connected");
    // simpledrm: a card with no render node.
    let bare = tree.path().join("class/drm/card0");
    fs::create_dir_all(bare.join("device/drm/card0")).unwrap_or_else(|e| panic!("{e}"));
    let gpus = scan(tree.path());
    assert_eq!(
        gpus,
        vec![
            gpu("card1", "pci-0000:10:00.0", 0),
            gpu("card10", "pci-0000:20:00.0", 1)
        ]
    );
}

#[test]
fn a_gpu_off_pci_is_named_by_its_render_node() {
    let tree = Tree::new("platform");
    tree.card("card0", "platform/fe000000.gpu", "renderD128")
        .card("card1", "pci0000:00/0000:01:00.0", "renderD129")
        .connector("card0-HDMI-A-1", "connected");
    let gpus = scan(tree.path());
    assert_eq!(gpus[0].device, DeviceName("renderD128".to_owned()));
    assert_eq!(decide(&gpus), Verdict::Pin(gpus[0].clone()));
}

#[test]
fn outputs_on_both_gpus_choose_nothing_and_a_missing_tree_is_empty() {
    let tree = Tree::new("split");
    tree.card("card1", "pci0000:00/0000:10:00.0", "renderD128")
        .card("card2", "pci0000:00/0000:01:00.0", "renderD129")
        .connector("card1-eDP-1", "connected")
        .connector("card2-HDMI-A-1", "connected");
    assert_eq!(
        decide(&scan(tree.path())),
        Verdict::Split(vec!["card1".to_owned(), "card2".to_owned()])
    );
    assert_eq!(
        scan(Path::new("/nonexistent-sill-sysfs")),
        Vec::<Gpu>::new()
    );
}
