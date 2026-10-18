//! The one place the brief sheet meets docket's wire shapes. Everything else in sill reads
//! [`BriefView`] and sends plain text; only these two functions name docket's types.

use docket_core::{ConfirmAnswer, ConfirmRequest, GrantScope};
use prov::ConfirmReceipt;

use super::brief::BriefView;

/// The brief the router put in `request`, if it is a brief sheet.
///
/// docket brief-confirm: `ConfirmRequest` gains `brief: Option<BriefSheet>` (title, text, flags
/// `{ span: (u32, u32), why: BriefWhy }`, pieces; spans are chars of `text`). When it lands, read
/// `request.brief` here: `Link/Address/Path/Order/Credentials` map one to one onto
/// [`super::brief::BriefKind`]. The origin line is not read from the wire (`words::origin_words`
/// builds it from `request.taint`). Until then no request is a brief sheet, so the plain sheet
/// is drawn and a brief can never be confirmed from here.
pub fn brief_of(request: &ConfirmRequest) -> Option<BriefView> {
    let _ = request;
    None
}

/// A yes for this once, carrying `written`: the text as the person left it in the field.
///
/// docket brief-confirm: `ConfirmAnswer::Allowed` gains `written: Option<BriefText>` (serde
/// default). When it lands, set it here from `written` (`BriefText` is bounded to 4000 bytes;
/// `BRIEF_MAX_BYTES` is the same bound). Until then the text cannot ride the answer, which
/// fails closed: the router ends a brief with no `written` as "Not confirmed".
pub fn allowed_once(receipt: ConfirmReceipt, written: Option<String>) -> ConfirmAnswer {
    let _ = written;
    ConfirmAnswer::Allowed {
        scope: GrantScope::Once,
        receipt,
    }
}
