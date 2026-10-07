//! The shell's sizes on the ladder nest concentrically and sit on the spacing grid
//! (design/29-SIZING.md section 6, R6 and R7).

use crate::tokens::control_center::CONTROL_CENTER;
use ds::style::tokens::shell_scale::SHELL_SCALE;
use ds::style::tokens::{
    control_size::ControlSize,
    size_scale::{WholePx, on_grid},
};

#[test]
fn the_shell_surfaces_nest_concentrically() {
    let cc = CONTROL_CENTER;
    // R6: the panel is its modules' radius plus the padding round them.
    assert_eq!(cc.panel_radius(), WholePx(18));
    // A level module: padding, header, gap, a Regular capsule, padding.
    assert_eq!(
        cc.level_module().0,
        2 * cc.padding.0 + cc.head.0 + cc.head_gap.0 + ControlSize::Regular.scale().height.0
    );
    assert_eq!(cc.tile.0, 2 * ControlSize::Large.scale().height.0);
    for step in [cc.padding, cc.gap, cc.module_radius, cc.head_gap] {
        assert!(on_grid(step.0), "{step:?} R7");
    }
    // The bar's items centre in it with the same space above and below.
    assert_eq!(SHELL_SCALE.item(), WholePx(22));
    assert_eq!((SHELL_SCALE.bar.0 - SHELL_SCALE.item().0) % 2, 0);
}
