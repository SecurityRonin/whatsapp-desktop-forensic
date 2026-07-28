//! End-to-end store parse over the real minted `model-storage` (tier-2 oracle):
//! deduplication by primary key, deleted-message recovery from tombstones, and
//! the merged timeline.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;
use std::path::PathBuf;
use whatsapp_desktop_core::open;

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/data/indexeddb/http_127.0.0.1_8731.indexeddb.leveldb"
    ))
}

#[test]
fn parses_deduplicated_records() {
    let store = open(&data_dir()).expect("open minted store");
    // 3 distinct messages (text, image, one deleted) — no superseded duplicates.
    assert_eq!(store.messages.len(), 3);
    let ids: HashSet<_> = store.messages.iter().map(|m| m.id.clone()).collect();
    assert_eq!(ids.len(), 3, "message ids are unique after dedup");
    assert_eq!(store.chats.len(), 1);
    assert_eq!(store.contacts.len(), 1);
}

#[test]
fn recovers_deleted_message_content_from_tombstone() {
    let store = open(&data_dir()).expect("open minted store");
    let deleted: Vec<_> = store.messages.iter().filter(|m| m.deleted).collect();
    assert_eq!(deleted.len(), 1, "exactly one deleted message");
    let dm = deleted[0];
    assert!(dm.id.ends_with("DELETEDMSG000001"));
    // Content survives in the superseded put record — recovered, not fabricated.
    assert_eq!(dm.from.as_deref(), Some("15551230000@c.us"));
    assert_eq!(dm.timestamp_secs, Some(1_596_233_333));
    assert_eq!(dm.kind.as_deref(), Some("chat"));
}

#[test]
fn timeline_is_ascending_and_flags_deleted() {
    let store = open(&data_dir()).expect("open minted store");
    assert_eq!(store.timeline.len(), 3, "three dated messages");
    assert!(
        store
            .timeline
            .windows(2)
            .all(|w| w[0].timestamp_secs <= w[1].timestamp_secs),
        "timeline ascending"
    );
    // Earliest event is the deleted message (t=22:08:53), flagged deleted.
    assert_eq!(store.timeline[0].timestamp_secs, 1_596_233_333);
    assert!(store.timeline[0].deleted);
    assert_eq!(store.timeline[0].rfc3339, "2020-07-31T22:08:53Z");
}
