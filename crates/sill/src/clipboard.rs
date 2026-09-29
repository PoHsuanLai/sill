//! `sill clipboard status|clear`: the report says whether history is kept, over which protocol, and
//! how many entries; never what they hold.

use sill_ipc::{
    ClipboardReport, ClipboardRequest, Failure, FailureKind, ReportClipMode, ReportClipProtocol,
    Response,
};
use sill_model::clipboard::{ClipProtocol, Clipboard, ClipboardCommand, HistoryMode};
use sill_services::Services;

/// The report for `history`.
pub fn report_of(history: &Clipboard) -> ClipboardReport {
    ClipboardReport {
        mode: match history.status.mode {
            HistoryMode::Off => ReportClipMode::Off,
            HistoryMode::Memory => ReportClipMode::Memory,
        },
        protocol: match history.status.protocol {
            ClipProtocol::NotConnected => ReportClipProtocol::NotConnected,
            ClipProtocol::Ext => ReportClipProtocol::Ext,
            ClipProtocol::Wlr => ReportClipProtocol::Wlr,
            ClipProtocol::Unsupported => ReportClipProtocol::Unsupported,
        },
        entries: u32::try_from(history.entries.len()).unwrap_or(u32::MAX),
    }
}

/// Answer `request`.
pub fn dispatch(services: &Services, request: ClipboardRequest) -> Response {
    match request {
        ClipboardRequest::Status => {
            Response::Clipboard(report_of(&services.clipboard.state.borrow()))
        }
        ClipboardRequest::Clear => match services.clipboard.control.send(ClipboardCommand::Clear) {
            Ok(()) => Response::Done,
            Err(error) => Response::Failed(Failure {
                kind: FailureKind::Internal,
                message: error.to_string(),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sill_ipc::{ClipboardReport, ReportClipMode, ReportClipProtocol};
    use sill_launcher::{ClipKey, UnixTime};
    use sill_model::clipboard::{
        ClipContent, ClipEntry, ClipProtocol, ClipStatus, ClipText, Clipboard, HistoryMode,
    };

    use super::report_of;

    #[test]
    fn the_report_counts_entries_and_says_nothing_of_them() {
        let history = Clipboard {
            status: ClipStatus {
                mode: HistoryMode::Memory,
                protocol: ClipProtocol::Wlr,
            },
            entries: vec![ClipEntry {
                key: ClipKey(1),
                content: ClipContent::Text(ClipText(Arc::from("private"))),
                copied: UnixTime(0),
            }],
        };
        let report = report_of(&history);
        assert_eq!(
            report,
            ClipboardReport {
                mode: ReportClipMode::Memory,
                protocol: ReportClipProtocol::Wlr,
                entries: 1,
            }
        );
        assert!(!format!("{report:?}").contains("private"));
    }
}
