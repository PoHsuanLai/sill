//! Every shell `Word` enum that is stored in a settings file writes the same word as its
//! attribute: a serde rename and a slug that drifted apart would make a saved choice unreadable.

use crate::battery::device_glyph::Device;
use crate::emoji::id::EmojiId;
use crate::widget::kind::{WidgetHost, WidgetSize};
use ds_core::testing::word_matches_serde;

#[test]
fn stored_vocabularies_serialise_as_their_slugs() {
    word_matches_serde::<Device>();
    word_matches_serde::<EmojiId>();
    word_matches_serde::<WidgetHost>();
    word_matches_serde::<WidgetSize>();
}
