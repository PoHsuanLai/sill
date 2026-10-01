use crate::keycap::{Attention, NoticeChange, notice_for, notice_text};

fn conflict(device: &str) -> Attention {
    Attention::Conflict {
        device: device.to_owned(),
    }
}

#[test]
fn the_notice_follows_what_keycap_needs() {
    let cases: Vec<(&str, Option<Attention>, Option<Attention>, NoticeChange)> = vec![
        ("nothing to nothing", None, None, NoticeChange::Keep),
        (
            "failure appears",
            None,
            Some(Attention::Failed),
            NoticeChange::Show(Attention::Failed),
        ),
        (
            "failure stays",
            Some(Attention::Failed),
            Some(Attention::Failed),
            NoticeChange::Keep,
        ),
        (
            "failure recovers",
            Some(Attention::Failed),
            None,
            NoticeChange::Withdraw,
        ),
        (
            "a pause replaces a conflict",
            Some(conflict("k1")),
            Some(Attention::Paused),
            NoticeChange::Show(Attention::Paused),
        ),
        (
            "another device conflicts",
            Some(conflict("k1")),
            Some(conflict("k2")),
            NoticeChange::Show(conflict("k2")),
        ),
        (
            "the same device conflicts",
            Some(conflict("k1")),
            Some(conflict("k1")),
            NoticeChange::Keep,
        ),
        (
            "the pause ends",
            Some(Attention::Paused),
            None,
            NoticeChange::Withdraw,
        ),
    ];
    for (case, previous, next, want) in cases {
        assert_eq!(notice_for(previous.as_ref(), next.as_ref()), want, "{case}");
    }
}

#[test]
fn each_attention_has_its_words() {
    let cases: Vec<(&str, Attention, &str, &str)> = vec![
        (
            "failed",
            Attention::Failed,
            "Mac keyboard layout stopped",
            "Keys work as on a standard keyboard. Run `sill keycap retry` to try again.",
        ),
        (
            "conflict",
            conflict("Apple Keyboard"),
            "Another keyboard program is running",
            "Apple Keyboard keeps its own mapping while it runs.",
        ),
        (
            "paused",
            Attention::Paused,
            "Mac keyboard layout paused",
            "Keys work as on a standard keyboard. Run `sill keycap resume` to turn it back on.",
        ),
    ];
    for (case, attention, summary, body) in cases {
        let notice = notice_text(&attention);
        assert_eq!(
            (notice.summary.as_str(), notice.body.as_str()),
            (summary, body),
            "{case}"
        );
    }
}
