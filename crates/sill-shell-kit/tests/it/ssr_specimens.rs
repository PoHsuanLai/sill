//! Every `*_ssr` module's specimens, checked once: each matches its golden (`DS_BLESS=1` rewrites
//! them, then read the diff), lints clean, and uses only `ds-` classes the stylesheet styles. A
//! new specimen is a row in its module's `specimens()`; a new module is a line here. Each module
//! keeps its own behaviour tests.

use crate::support::specimens::{Specimen, mismatches, offences};

fn all() -> Vec<Specimen> {
    [
        crate::control_center_ssr::specimens(),
        crate::date_picker_ssr::specimens(),
        crate::emoji_ssr::specimens(),
        crate::level_ssr::specimens(),
        crate::lock_switcher_ssr::specimens(),
        crate::month_grid_density_ssr::specimens(),
        crate::month_grid_ssr::specimens(),
        crate::notifications_ssr::specimens(),
        crate::shot_thumbnail_ssr::specimens(),
        crate::surface_materials_ssr::specimens(),
        crate::user_picture_ssr::specimens(),
        crate::widget_contract_ssr::specimens(),
        crate::widgets_ssr::specimens(),
    ]
    .concat()
}

#[test]
fn every_specimen_matches_its_golden() {
    let failures = mismatches(&all());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_specimen_lints_clean_and_every_class_is_styled() {
    let failures = offences(&all());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
