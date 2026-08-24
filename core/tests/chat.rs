//! Chat extraction over the tier-2 minted store.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use whatsapp_desktop_core::{read_dir, Chat};

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/data/indexeddb/http_127.0.0.1_8731.indexeddb.leveldb"
    ))
}

fn chats() -> Vec<Chat> {
    read_dir(&data_dir())
        .unwrap()
        .iter()
        .filter(|r| r.object_store.as_deref() == Some("chat") && !r.deleted)
        .filter_map(Chat::from_record)
        .collect()
}

#[test]
fn extracts_chat_row() {
    let all = chats();
    assert_eq!(all.len(), 1, "one chat minted");
    let c = &all[0];
    assert_eq!(c.id, "15551239999@c.us");
    assert_eq!(c.name.as_deref(), Some("Alice"));
    assert_eq!(c.timestamp_secs, Some(1_596_233_500));
    assert_eq!(c.unread_count, Some(2));
}
