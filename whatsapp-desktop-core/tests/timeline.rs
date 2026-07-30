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
fn equal_timestamps_break_the_tie_by_message_id() {
    // Two messages at the SAME second must be ordered deterministically by
    // message id (ascending), exercising the `.then_with` tie-break comparator.
    let t = 1_596_233_451;
    let messages = vec![msg("bbb", Some(t), "chat"), msg("aaa", Some(t), "chat")];
    let tl: Vec<TimelineEntry> = TimelineEntry::build_timeline(&messages);
    assert_eq!(tl.len(), 2);
    assert_eq!(tl[0].message_id, "aaa");
    assert_eq!(tl[1].message_id, "bbb");
    assert_eq!(tl[0].timestamp_secs, tl[1].timestamp_secs);
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

#[test]
fn an_implausible_timestamp_never_becomes_a_timeline_row() {
    // Zero is not the only value that is no send time. A NEGATIVE `t` renders as a
    // pre-1970 date, a pre-2009 `t` predates WhatsApp itself, and a millisecond
    // value read as seconds lands tens of thousands of years out — each would sit
    // on a forensic timeline as a fabricated event. Only a plausible time is datable.
    let messages = vec![
        msg("negative", Some(-1), "chat"),
        msg("pre_whatsapp", Some(1_000_000_000), "chat"), // 2001-09-09
        msg("millis_as_secs", Some(1_596_233_451_000), "chat"), // year 52560
        msg("dated", Some(1_596_233_451), "chat"),
    ];
    let tl: Vec<TimelineEntry> = TimelineEntry::build_timeline(&messages);
    assert_eq!(tl.len(), 1, "only the plausible send time is datable");
    assert_eq!(tl[0].message_id, "dated");
    assert!(
        tl.iter().all(|e| !e.rfc3339.starts_with("19")),
        "no pre-2000 row may enter a forensic timeline"
    );
}

#[test]
fn a_zero_timestamp_never_becomes_a_1970_timeline_row() {
    // The pure `epoch_secs_to_rfc3339(0) == "1970-01-01T00:00:00Z"` above is the
    // correct answer for the *conversion*. What must never happen is a message
    // reaching the timeline with the zero sentinel as its event time: 0 is not a
    // WhatsApp send time, so the entry is not datable and is omitted.
    let messages = vec![
        msg("zero", Some(0), "chat"),
        msg("dated", Some(1_596_233_451), "chat"),
    ];
    let tl: Vec<TimelineEntry> = TimelineEntry::build_timeline(&messages);
    assert_eq!(tl.len(), 1, "the t=0 message is not datable");
    assert_eq!(tl[0].message_id, "dated");
    assert!(
        tl.iter().all(|e| e.rfc3339 != "1970-01-01T00:00:00Z"),
        "no 1970 row may enter a forensic timeline"
    );
}
