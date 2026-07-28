//! `build_timeline` ordering contract (deterministic inputs) + an epoch→RFC3339
//! Known-Answer-Test (independent oracle: Python `datetime`, UTC).
//!
//! The end-to-end timeline over the real minted store (deduplicated, with
//! deleted-message recovery) is asserted in `tests/store.rs`; here we pin the
//! pure ordering/rendering function.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use whatsapp_desktop_core::{epoch_secs_to_rfc3339, Message, MessageBody, TimelineEntry};

fn msg(id: &str, t: Option<i64>, kind: &str) -> Message {
    Message {
        id: id.to_string(),
        timestamp_secs: t,
        from: None,
        to: None,
        kind: Some(kind.to_string()),
        ack: None,
        notify_name: None,
        body: MessageBody::None,
        media: None,
        deleted: false,
        seq: 0,
    }
}

#[test]
fn epoch_to_rfc3339_matches_independent_oracle() {
    // Ground truth from Python datetime (UTC) — an independent implementation.
    assert_eq!(epoch_secs_to_rfc3339(0), "1970-01-01T00:00:00Z");
    assert_eq!(epoch_secs_to_rfc3339(1_596_233_451), "2020-07-31T22:10:51Z");
    assert_eq!(epoch_secs_to_rfc3339(1_596_233_500), "2020-07-31T22:11:40Z");
    // Leap day, and a pre-epoch negative.
    assert_eq!(epoch_secs_to_rfc3339(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(epoch_secs_to_rfc3339(-1), "1969-12-31T23:59:59Z");
}

#[test]
fn timeline_sorts_ascending_and_renders_time() {
    let messages = vec![
        msg("later", Some(1_596_233_500), "image"),
        msg("earlier", Some(1_596_233_451), "chat"),
        msg("undated", None, "chat"), // omitted — no time to place it
    ];
    let tl: Vec<TimelineEntry> = TimelineEntry::build_timeline(&messages);
    assert_eq!(tl.len(), 2, "undated message is omitted");
    assert_eq!(tl[0].message_id, "earlier");
    assert_eq!(tl[0].rfc3339, "2020-07-31T22:10:51Z");
    assert_eq!(tl[1].message_id, "later");
    assert_eq!(tl[1].kind.as_deref(), Some("image"));
    assert!(tl[0].timestamp_secs < tl[1].timestamp_secs);
}
