//! Merged message timeline over the tier-2 minted store, plus an epoch→RFC3339
//! Known-Answer-Test (independent oracle: Python `datetime`, UTC).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use whatsapp_desktop_core::{epoch_secs_to_rfc3339, read_dir, Message, TimelineEntry};

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/data/indexeddb/http_127.0.0.1_8731.indexeddb.leveldb"
    ))
}

fn live_messages() -> Vec<Message> {
    read_dir(&data_dir())
        .unwrap()
        .iter()
        .filter(|r| r.object_store.as_deref() == Some("message") && !r.deleted)
        .filter_map(Message::from_record)
        .collect()
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
fn timeline_is_time_ordered_with_rendered_timestamps() {
    let tl: Vec<TimelineEntry> = TimelineEntry::build_timeline(&live_messages());
    // Two live, dated messages (text @ 22:10:51, image @ 22:11:40).
    assert_eq!(tl.len(), 2);
    assert!(
        tl[0].timestamp_secs <= tl[1].timestamp_secs,
        "timeline must be ascending by time"
    );
    assert_eq!(tl[0].timestamp_secs, 1_596_233_451);
    assert_eq!(tl[0].rfc3339, "2020-07-31T22:10:51Z");
    assert!(tl[0].message_id.ends_with("3EB0A1B2C3D4E5F6"));
    assert_eq!(tl[1].rfc3339, "2020-07-31T22:11:40Z");
    assert_eq!(tl[1].kind.as_deref(), Some("image"));
}
