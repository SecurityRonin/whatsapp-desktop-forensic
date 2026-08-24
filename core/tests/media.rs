//! Media-metadata extraction over the tier-2 minted store.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use whatsapp_desktop_core::{read_dir, Message};

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
fn image_message_carries_media_metadata() {
    let m = live_messages()
        .into_iter()
        .find(|m| m.id.ends_with("ABCDEF0123456789"))
        .expect("media message present");
    let media = m.media.expect("image message has media metadata");
    assert_eq!(media.mimetype.as_deref(), Some("image/jpeg"));
    assert_eq!(
        media.filehash.as_deref(),
        Some("n3v9J0k2Q1r4S5t6U7v8W9x0Y1z2A3b4C5d6E7f8=")
    );
    assert_eq!(
        media.media_key.as_deref(),
        Some("aGVsbG8td29ybGQtbWVkaWEta2V5LWJhc2U2NC0xMjM0NTY=")
    );
    assert_eq!(
        media.direct_path.as_deref(),
        Some("/v/t62.7118-24/12345678_9012345_67890.enc")
    );
    assert_eq!(media.size, Some(34567));
    assert_eq!(media.width, Some(1280));
    assert_eq!(media.height, Some(720));
}

#[test]
fn text_message_has_no_media_metadata() {
    let m = live_messages()
        .into_iter()
        .find(|m| m.id.ends_with("3EB0A1B2C3D4E5F6"))
        .expect("text message present");
    assert!(m.media.is_none());
}
