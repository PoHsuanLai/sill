//! What the daemon sends the compositor for the Spaces bars, given where the bars' cards lie and
//! what the desktop shows. Pure.

use super::{
    BarLayouts, BarPictures, DesktopCommand, DesktopView, ThumbAsks, target_commands, thumbs_wanted,
};

/// What has been sent for the bars: the requests for pictures waiting or answered, and the drop
/// targets the compositor holds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outbox {
    asks: ThumbAsks,
    sent: BarLayouts,
}

/// The commands the compositor needs now: the drop targets that changed, then a request for the
/// picture of each Space that is due one ([`thumbs_wanted`]). The outbox that follows is returned
/// with them.
pub fn outgoing(
    outbox: Outbox,
    layouts: &BarLayouts,
    view: &DesktopView,
    pictures: &BarPictures,
) -> (Outbox, Vec<DesktopCommand>) {
    let mut commands = target_commands(&outbox.sent, layouts);
    let (asks, requests) = thumbs_wanted(&outbox.asks, layouts, view, pictures);
    commands.extend(
        requests
            .into_iter()
            .map(|request| DesktopCommand::CaptureSpace {
                output: request.output,
                space: request.space,
                size: request.size,
            }),
    );
    let next = Outbox {
        asks,
        sent: layouts.clone(),
    };
    (next, commands)
}
