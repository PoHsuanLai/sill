//! The one place the brief sheet meets docket's wire shapes. Everything else in sill reads
//! [`BriefView`] and sends plain text; only these two functions name docket's types.

use docket_core::{BriefText, BriefWhy, ConfirmAnswer, ConfirmRequest, GrantScope};
use prov::ConfirmReceipt;

use super::brief::{BriefFlag, BriefKind, BriefSpan, BriefView};

/// The brief the router put in `request`, if it is a brief sheet. The origin line is not read
/// from here: `words::origin_words` builds it from `request.taint`.
pub fn brief_of(request: &ConfirmRequest) -> Option<BriefView> {
    let sheet = request.brief.as_ref()?;
    Some(BriefView {
        text: sheet.text.as_str().to_owned(),
        flags: sheet.flags.iter().map(flag_of).collect(),
        pieces: sheet.pieces.iter().map(|p| p.as_str().to_owned()).collect(),
    })
}

fn flag_of(flag: &docket_core::BriefFlag) -> BriefFlag {
    let (start, end) = flag.span;
    BriefFlag {
        span: BriefSpan {
            start: start as usize,
            end: end as usize,
        },
        kind: kind_of(flag.why),
    }
}

fn kind_of(why: BriefWhy) -> BriefKind {
    match why {
        BriefWhy::Link => BriefKind::Link,
        BriefWhy::Address => BriefKind::Address,
        BriefWhy::Path => BriefKind::Path,
        BriefWhy::Order => BriefKind::Order,
        BriefWhy::Credentials => BriefKind::Credentials,
    }
}

/// A yes for this once, carrying `written`: the text as the person left it in the field. Text
/// the router would refuse (empty, or over its bound) is sent as none, which the router ends as
/// "Not confirmed": a brief is never confirmed with words other than the person's.
pub fn allowed_once(receipt: ConfirmReceipt, written: Option<String>) -> ConfirmAnswer {
    ConfirmAnswer::Allowed {
        scope: GrantScope::Once,
        receipt,
        written: written.and_then(|text| BriefText::parse(&text).ok()),
    }
}
