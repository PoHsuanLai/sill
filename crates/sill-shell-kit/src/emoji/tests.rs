use super::id::EmojiId;
use super::play::animation;
use super::sheet::{MANIFEST_JSON, SheetPx, durations, png, position, timing};
use ds::base::word::Word;
use std::time::Duration;

#[test]
fn the_manifest_lists_every_emoji_in_order() {
    let manifest: serde_json::Value = serde_json::from_str(MANIFEST_JSON).expect("manifest");
    let listed: Vec<&str> = manifest["emoji"]
        .as_array()
        .expect("emoji")
        .iter()
        .filter_map(|entry| entry["slug"].as_str())
        .collect();
    let ours: Vec<&str> = EmojiId::ALL.iter().map(|emoji| emoji.slug()).collect();
    assert_eq!(listed, ours);
    assert_eq!(manifest["licence"], "CC-BY-4.0");
}

#[test]
fn every_sheet_is_its_grid_of_frames() {
    for emoji in EmojiId::ALL.iter().copied() {
        let grid = timing(emoji);
        assert!(grid.frames > 1, "{emoji:?} is still");
        assert!(grid.frames <= grid.columns * grid.rows, "{emoji:?}");
        assert!(
            grid.frames > grid.columns * (grid.rows - 1),
            "{emoji:?}: an empty row"
        );
        for px in [SheetPx::Px128, SheetPx::Px256] {
            let image = image::load_from_memory(png(emoji, px)).expect("png");
            assert_eq!(
                (image.width(), image.height()),
                (
                    u32::from(grid.columns) * px.px(),
                    u32::from(grid.rows) * px.px()
                ),
                "{emoji:?} {px:?}"
            );
            // The renderer's texture limit.
            assert!(image.width() <= 4096 && image.height() <= 4096);
        }
    }
}

#[test]
fn the_id_is_stored_as_its_slug() {
    for emoji in EmojiId::ALL.iter().copied() {
        let json = serde_json::to_string(&emoji).expect("serialise");
        assert_eq!(json, format!("\"{}\"", emoji.slug()));
        let back: EmojiId = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, emoji);
    }
    assert_eq!(EmojiId::default(), EmojiId::Blush);
}

#[test]
fn a_frame_is_a_share_of_the_sheet() {
    let grid = timing(EmojiId::Wink);
    assert_eq!(position(grid, 0).0, "0% 0%");
    let (last, fit) = position(grid, grid.columns - 1);
    assert_eq!(last, "100% 0%");
    assert_eq!(fit, format!("{}% {}%", grid.columns * 100, grid.rows * 100));
    let (below, _) = position(grid, grid.columns);
    assert!(below.starts_with("0% "), "{below}");
}

#[test]
fn an_animation_is_every_frame_once_at_the_assets_own_durations() {
    for emoji in EmojiId::ALL.iter().copied() {
        let played = animation(emoji);
        let frames = timing(emoji).frames;
        assert_eq!(played.len(), usize::from(frames), "{emoji:?}");
        assert!(
            played
                .iter()
                .enumerate()
                .all(|(at, held)| usize::from(held.frame) == at),
            "{emoji:?}: frames in order from the rest pose"
        );
        let total: Duration = played.iter().map(|held| held.hold).sum();
        let assets: Duration = durations(emoji).into_iter().sum();
        assert_eq!(total, assets, "{emoji:?}: the asset's own timing");
    }
}
