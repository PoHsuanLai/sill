//! The control center's geometry on the size ladder (design/29-SIZING.md sections 7 and 13): a
//! 320 panel, 10 of padding and 10 between modules, modules rounded 8 inside a panel rounded 18
//! (8 + 10, R6), a toggle tile two Large controls tall, and a level module that is its padding,
//! a 16 header, a 6 gap and a Regular capsule. sill sizes its popup from these (R9) rather than
//! from its own copies.

use ds::base::word::Word;
use ds::style::tokens::control_size::ControlSize;
use ds::style::tokens::size_scale::WholePx;
use ds::style::tokens::token::{CssValue, Token, TokenScope};

/// The control center's sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControlCenterScale {
    /// The panel's width, 320.
    pub width: WholePx,
    /// Inside the panel's edge and inside a module, 10.
    pub padding: WholePx,
    /// Between modules, 10.
    pub gap: WholePx,
    /// A module's radius, 8.
    pub module_radius: WholePx,
    /// A toggle tile's height, 64 (two Large controls).
    pub tile: WholePx,
    /// A module header's line, 16.
    pub head: WholePx,
    /// Between a module's header and its control, 6.
    pub head_gap: WholePx,
}

/// The settled numbers (design/29 section 13).
pub const CONTROL_CENTER: ControlCenterScale = ControlCenterScale {
    width: WholePx(320),
    padding: WholePx(10),
    gap: WholePx(10),
    module_radius: WholePx(8),
    tile: WholePx(64),
    head: WholePx(16),
    head_gap: WholePx(6),
};

impl ControlCenterScale {
    /// The panel's radius: a module's plus the padding round it (R6), 18.
    pub fn panel_radius(self) -> WholePx {
        WholePx(self.module_radius.0 + self.padding.0)
    }

    /// A level module's height: padding, header, gap, a Regular capsule, padding; 70.
    pub fn level_module(self) -> WholePx {
        WholePx(
            2 * self.padding.0
                + self.head.0
                + self.head_gap.0
                + ControlSize::Regular.scale().height.0,
        )
    }
}

/// One of the control center's sizes, as a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word, Token)]
#[token(prefix = "cc-", kind = fixed, css = control_center_css)]
pub enum ControlCenterSize {
    /// `--cc-width`: the panel's width.
    #[token(name = "width")]
    Width,
    /// `--cc-pad`: the padding inside the panel and a module.
    #[token(name = "pad")]
    Padding,
    /// `--cc-gap`: between modules.
    #[token(name = "gap")]
    Gap,
    /// `--cc-module-r`: a module's radius.
    #[token(name = "module-r")]
    ModuleRadius,
    /// `--cc-panel-r`: the panel's radius.
    #[token(name = "panel-r")]
    PanelRadius,
    /// `--cc-tile-h`: a toggle tile's height.
    #[token(name = "tile-h")]
    Tile,
    /// `--cc-head`: a module header's line.
    #[token(name = "head")]
    Head,
    /// `--cc-head-gap`: between a module's header and its control.
    #[token(name = "head-gap")]
    HeadGap,
    /// `--cc-level-h`: a level module's height.
    #[token(name = "level-h")]
    LevelModule,
}

/// The size a control center token holds, as the stylesheet writes it.
fn control_center_css(token: ControlCenterSize, _scope: TokenScope) -> CssValue {
    let scale = CONTROL_CENTER;
    let size = match token {
        ControlCenterSize::Width => scale.width,
        ControlCenterSize::Padding => scale.padding,
        ControlCenterSize::Gap => scale.gap,
        ControlCenterSize::ModuleRadius => scale.module_radius,
        ControlCenterSize::PanelRadius => scale.panel_radius(),
        ControlCenterSize::Tile => scale.tile,
        ControlCenterSize::Head => scale.head,
        ControlCenterSize::HeadGap => scale.head_gap,
        ControlCenterSize::LevelModule => scale.level_module(),
    };
    CssValue::computed(size.css())
}
