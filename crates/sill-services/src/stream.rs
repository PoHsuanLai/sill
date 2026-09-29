//! Awaiting a stream's next item without `futures`' `StreamExt`: the live loops' `tokio::select!`
//! branches over D-Bus signal streams.

use std::pin::Pin;

use zbus::export::futures_core::Stream;

/// The next item of one `Unpin` stream; `None` once it has ended.
pub(crate) async fn next<S>(stream: &mut S) -> Option<S::Item>
where
    S: Stream + Unpin,
{
    std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}

/// [`next`] for a stream that might not exist: `None` never resolves, so the `tokio::select!`
/// branch it backs simply never fires instead of needing its own guard.
pub(crate) async fn next_opt<S>(stream: &mut Option<S>) -> Option<S::Item>
where
    S: Stream + Unpin,
{
    match stream {
        Some(stream) => next(stream).await,
        None => std::future::pending().await,
    }
}
