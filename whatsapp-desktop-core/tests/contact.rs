//! Contact extraction over the tier-2 minted store.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use whatsapp_desktop_core::{read_dir, Contact};

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/data/indexeddb/http_127.0.0.1_8731.indexeddb.leveldb"
    ))
}

fn contacts() -> Vec<Contact> {
    read_dir(&data_dir())
        .unwrap()
        .iter()
        .filter(|r| r.object_store.as_deref() == Some("contact") && !r.deleted)
        .filter_map(Contact::from_record)
        .collect()
}

#[test]
fn extracts_contact_row() {
    let all = contacts();
    assert_eq!(all.len(), 1, "one contact minted");
    let c = &all[0];
    assert_eq!(c.id, "15551239999@c.us");
    assert_eq!(c.name.as_deref(), Some("Alice Example"));
    assert_eq!(c.push_name.as_deref(), Some("Alice"));
    assert_eq!(c.notify_name.as_deref(), Some("Alice"));
    assert_eq!(c.short_name.as_deref(), Some("Alice"));
}
